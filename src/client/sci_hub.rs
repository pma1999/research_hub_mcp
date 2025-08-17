use crate::client::{
    mirror::MirrorManager,
    rate_limiter::{AdaptiveRateLimiter, RateLimitConfig},
    Doi, HttpClientConfig, PaperMetadata,
};
use crate::{Config, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, error, info, warn};
use url::Url;

/// Response from research source containing paper information
#[derive(Debug, Clone)]
pub struct ResearchResponse {
    /// Paper metadata
    pub metadata: PaperMetadata,
    /// Source mirror that provided the response
    pub source_mirror: Url,
    /// Response time
    pub response_time: Duration,
    /// Whether the paper was found
    pub found: bool,
}

/// User agent strings for rotation
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
];

/// Main research client with comprehensive error handling and resilience
pub struct ResearchClient {
    http_client: Client,
    mirror_manager: Arc<MirrorManager>,
    rate_limiter: Arc<RwLock<AdaptiveRateLimiter>>,
    config: Arc<RwLock<Config>>,
    user_agent_index: Arc<RwLock<usize>>,
}

impl ResearchClient {
    /// Create a new research client with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        let http_config = HttpClientConfig::default();
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.research_source.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10))
            .gzip(true)
            .user_agent(&http_config.user_agent);

        // Configure proxy if provided
        if let Some(proxy_url) = &http_config.proxy {
            let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| crate::Error::InvalidInput {
                field: "proxy".to_string(),
                reason: format!("Invalid proxy URL: {e}"),
            })?;
            client_builder = client_builder.proxy(proxy);
        }

        // SSL/TLS configuration
        if http_config.danger_accept_invalid_certs {
            warn!("DANGER: Accepting invalid SSL certificates!");
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let http_client = client_builder
            .build()
            .map_err(|e| crate::Error::Service(format!("Failed to create HTTP client: {e}")))?;

        // Initialize mirror manager
        let mirror_manager = Arc::new(MirrorManager::new(
            config.research_source.endpoints.clone(),
            http_client.clone(),
        )?);

        // Initialize rate limiter
        let rate_config = RateLimitConfig {
            requests_per_second: config.research_source.rate_limit_per_sec,
            adaptive: true,
            min_rate: 1,
            max_rate: config.research_source.rate_limit_per_sec * 2,
        };
        let rate_limiter = Arc::new(RwLock::new(AdaptiveRateLimiter::new(rate_config)));

        info!(
            "Initialized Sci-Hub client with {} mirrors",
            config.research_source.endpoints.len()
        );

        Ok(Self {
            http_client,
            mirror_manager,
            rate_limiter,
            config: Arc::new(RwLock::new(config)),
            user_agent_index: Arc::new(RwLock::new(0)),
        })
    }

    /// Search for a paper by DOI
    pub async fn search_by_doi(&self, doi: &Doi) -> Result<ResearchResponse> {
        self.search_paper(&doi.to_string()).await
    }

    /// Search for a paper by title
    pub async fn search_by_title(&self, title: &str) -> Result<ResearchResponse> {
        if title.trim().is_empty() {
            return Err(crate::Error::InvalidInput {
                field: "title".to_string(),
                reason: "title cannot be empty".to_string(),
            });
        }

        self.search_paper(title).await
    }

    /// Internal method to search for a paper using various strategies
    async fn search_paper(&self, query: &str) -> Result<ResearchResponse> {
        let start_time = SystemTime::now();

        // Apply rate limiting
        self.rate_limiter.write().await.acquire().await;

        // Perform search with retry logic
        let response = self.perform_search(query).await?;

        let response_time = start_time.elapsed().unwrap_or(Duration::ZERO);

        // Record successful response time for adaptive rate limiting
        {
            let mut rate_limiter = self.rate_limiter.write().await;
            rate_limiter.record_response_time(response_time);
        }

        // Mark mirror as successful
        self.mirror_manager
            .mark_mirror_success(&response.source_mirror, response_time)
            .await;

        debug!(
            "Search completed successfully in {}ms",
            response_time.as_millis()
        );
        Ok(response)
    }

    /// Perform the actual search with retry logic
    async fn perform_search(&self, query: &str) -> Result<ResearchResponse> {
        let retry_strategy = ExponentialBackoff::from_millis(1000)
            .max_delay(Duration::from_secs(30))
            .take(3); // Maximum 3 retries

        let config = self.config.read().await;
        let max_retries = config.research_source.max_retries;
        drop(config);

        let query_owned = query.to_string(); // Take ownership to avoid lifetime issues

        Retry::spawn(retry_strategy.take(max_retries as usize), || {
            self.try_search_on_available_mirror(&query_owned)
        })
        .await
    }

    /// Try to search on an available mirror
    async fn try_search_on_available_mirror(&self, query: &str) -> Result<ResearchResponse> {
        // Get next available mirror
        let mirror = self.mirror_manager.get_next_mirror().await.ok_or_else(|| {
            crate::Error::ServiceUnavailable {
                service: "Sci-Hub".to_string(),
                reason: "No healthy mirrors available".to_string(),
            }
        })?;

        debug!("Attempting search on mirror: {}", mirror.url);

        // Rotate user agent
        let user_agent = self.get_next_user_agent().await;

        // Build search URL
        let search_url = Self::build_search_url(&mirror.url, query);

        let start_time = SystemTime::now();

        // Make HTTP request
        let response = self
            .http_client
            .get(&search_url)
            .header("User-Agent", user_agent)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Accept-Encoding", "gzip, deflate")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed for mirror {}: {}", mirror.url, e);
                crate::Error::Service(format!("Request failed: {e}"))
            })?;

        let response_time = start_time.elapsed().unwrap_or(Duration::ZERO);

        if !response.status().is_success() {
            let status = response.status();
            self.mirror_manager.mark_mirror_failed(&mirror.url).await;
            return Err(crate::Error::SciHub {
                code: status.as_u16(),
                message: format!("HTTP error from {}", mirror.url),
            });
        }

        // Parse response
        let html_content = response
            .text()
            .await
            .map_err(|e| crate::Error::Service(format!("Failed to read response body: {e}")))?;

        let metadata = Self::parse_sci_hub_response(&html_content, query)?;

        Ok(ResearchResponse {
            metadata,
            source_mirror: mirror.url,
            response_time,
            found: true, // If we got here, we found something
        })
    }

    /// Build search URL for a given mirror and query
    fn build_search_url(mirror_url: &Url, query: &str) -> String {
        let encoded_query = urlencoding::encode(query);

        // For now, use the direct DOI/title lookup pattern
        format!(
            "{}/{}",
            mirror_url.as_str().trim_end_matches('/'),
            encoded_query
        )
    }

    /// Parse Sci-Hub HTML response to extract paper metadata
    fn parse_sci_hub_response(html: &str, original_query: &str) -> Result<PaperMetadata> {
        let document = Html::parse_document(html);

        // Try to find PDF download link
        let pdf_selector = Selector::parse(
            "a[href*='.pdf'], iframe[src*='.pdf'], embed[src*='.pdf']",
        )
        .map_err(|e| crate::Error::Parse {
            context: "CSS selector".to_string(),
            message: format!("Invalid CSS selector: {e}"),
        })?;

        let pdf_url = document
            .select(&pdf_selector)
            .find_map(|element| {
                element
                    .value()
                    .attr("href")
                    .or_else(|| element.value().attr("src"))
            })
            .map(|url| {
                // Convert relative URLs to absolute
                if url.starts_with("http") {
                    url.to_string()
                } else {
                    format!("https:{}", url.trim_start_matches('/'))
                }
            });

        // Try to extract title
        let title_selectors = [
            "h1",
            "#title",
            ".title",
            "title",
            "meta[name='citation_title']",
            "meta[property='og:title']",
        ];

        let title = title_selectors.iter().find_map(|selector| {
            Selector::parse(selector).ok().and_then(|sel| {
                document.select(&sel).next().and_then(|element| {
                    if selector.contains("meta") {
                        element.value().attr("content").map(ToString::to_string)
                    } else {
                        let text = element.text().collect::<String>();
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    }
                })
            })
        });

        // Try to extract authors
        let author_selectors = [
            "meta[name='citation_author']",
            "meta[name='author']",
            ".authors",
            ".author",
        ];

        let mut authors = Vec::new();
        for selector_str in &author_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if selector_str.contains("meta") {
                        if let Some(author) = element.value().attr("content") {
                            authors.push(author.trim().to_string());
                        }
                    } else {
                        let author_text = element.text().collect::<String>();
                        if !author_text.trim().is_empty() {
                            authors.push(author_text.trim().to_string());
                        }
                    }
                }
            }
            if !authors.is_empty() {
                break;
            }
        }

        // Create metadata
        let mut metadata = PaperMetadata::new(original_query.to_string());
        metadata.title = title;
        metadata.authors = authors;
        metadata.pdf_url = pdf_url;

        debug!(
            "Parsed metadata: title={:?}, authors={}, pdf_url={:?}",
            metadata.title,
            metadata.authors.len(),
            metadata.pdf_url.is_some()
        );

        Ok(metadata)
    }

    /// Get the next user agent for rotation
    async fn get_next_user_agent(&self) -> &'static str {
        let mut index = self.user_agent_index.write().await;
        let user_agent = USER_AGENTS[*index % USER_AGENTS.len()];
        *index = (*index + 1) % USER_AGENTS.len();
        user_agent
    }

    /// Perform health checks on all mirrors
    pub async fn health_check_mirrors(&self) {
        self.mirror_manager.health_check_all().await;
    }

    /// Get status of all mirrors
    pub async fn get_mirror_status(&self) -> Vec<crate::client::mirror::Mirror> {
        self.mirror_manager.get_mirror_status().await
    }

    /// Get current rate limiting status
    pub async fn get_rate_limit_info(&self) -> (u32, Option<Duration>) {
        let rate_limiter = self.rate_limiter.read().await;
        (
            rate_limiter.current_rate(),
            rate_limiter.average_response_time(),
        )
    }

    /// Update configuration (for hot reload)
    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        {
            let mut config = self.config.write().await;

            // Update rate limiter if rate changed
            if config.research_source.rate_limit_per_sec
                != new_config.research_source.rate_limit_per_sec
            {
                let rate_config = RateLimitConfig {
                    requests_per_second: new_config.research_source.rate_limit_per_sec,
                    adaptive: true,
                    min_rate: 1,
                    max_rate: new_config.research_source.rate_limit_per_sec * 2,
                };
                *self.rate_limiter.write().await = AdaptiveRateLimiter::new(rate_config);
                info!(
                    "Updated rate limit to {} requests/sec",
                    new_config.research_source.rate_limit_per_sec
                );
            }

            *config = new_config;
        }
        Ok(())
    }

    /// Test connectivity to Sci-Hub mirrors
    pub async fn test_connectivity(&self) -> Result<Vec<(Url, bool, Option<Duration>)>> {
        let mirrors = self.mirror_manager.get_mirror_status().await;
        let mut results = Vec::new();

        for mirror in mirrors {
            let start_time = SystemTime::now();
            let is_reachable = self
                .http_client
                .head(mirror.url.as_str())
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false);

            let response_time = if is_reachable {
                Some(start_time.elapsed().unwrap_or(Duration::ZERO))
            } else {
                None
            };

            results.push((mirror.url, is_reachable, response_time));
        }

        Ok(results)
    }
}

// HTTP client error conversion is handled by the HttpClient variant

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ResearchSourceConfig};

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.research_source = ResearchSourceConfig {
            endpoints: vec!["https://sci-hub.se".to_string()],
            rate_limit_per_sec: 1,
            timeout_secs: 30,
            max_retries: 2,
        };
        config
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = create_test_config();
        let client = ResearchClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_doi_validation() {
        let valid_doi = Doi::new("10.1038/nature12373");
        assert!(valid_doi.is_ok());

        let invalid_doi = Doi::new("");
        assert!(invalid_doi.is_err());

        let invalid_doi2 = Doi::new("not-a-doi");
        assert!(invalid_doi2.is_err());
    }

    #[test]
    fn test_search_url_building() {
        let mirror_url = Url::parse("https://sci-hub.se").unwrap();
        let url = ResearchClient::build_search_url(&mirror_url, "10.1038/nature12373");
        assert!(url.contains("10.1038"));
    }

    #[test]
    fn test_html_parsing() {
        let html = r#"
            <html>
            <head><title>Test Paper</title></head>
            <body>
                <h1>A Great Scientific Paper</h1>
                <a href="https://example.com/paper.pdf">Download PDF</a>
            </body>
            </html>
        "#;

        let metadata = ResearchClient::parse_sci_hub_response(html, "10.1038/test").unwrap();
        assert!(metadata.title.is_some());
        assert!(metadata.pdf_url.is_some());
    }
}
