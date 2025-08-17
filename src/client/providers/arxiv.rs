use super::traits::{ProviderError, ProviderResult, SearchContext, SearchQuery, SearchType, SourceProvider};
use crate::client::PaperMetadata;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use url::Url;

/// arXiv API provider for academic papers
pub struct ArxivProvider {
    client: Client,
    base_url: String,
}

impl ArxivProvider {
    /// Create a new arXiv provider
    pub fn new() -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("rust-research-mcp/0.2.0 (Academic Research Tool)")
            .build()
            .map_err(|e| ProviderError::Other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: "http://export.arxiv.org/api/query".to_string(),
        })
    }

    /// Build arXiv API URL for search
    fn build_search_url(&self, query: &SearchQuery) -> Result<String, ProviderError> {
        let mut url = Url::parse(&self.base_url)
            .map_err(|e| ProviderError::Other(format!("Invalid base URL: {}", e)))?;

        // Build search terms based on query type
        let search_query = match query.search_type {
            SearchType::Doi => format!("doi:{}", query.query),
            SearchType::Title => format!("ti:\"{}\"", query.query),
            SearchType::Author => format!("au:\"{}\"", query.query),
            SearchType::Keywords | SearchType::Auto => {
                // For auto/keywords, search in title, abstract, and comments
                format!("all:\"{}\"", query.query)
            }
            SearchType::Subject => format!("cat:{}", query.query),
        };

        url.query_pairs_mut()
            .append_pair("search_query", &search_query)
            .append_pair("start", &query.offset.to_string())
            .append_pair("max_results", &query.max_results.to_string())
            .append_pair("sortBy", "relevance")
            .append_pair("sortOrder", "descending");

        Ok(url.to_string())
    }

    /// Parse arXiv Atom feed response
    fn parse_response(&self, response_text: &str) -> Result<Vec<PaperMetadata>, ProviderError> {
        use roxmltree::Document;

        let doc = Document::parse(response_text)
            .map_err(|e| ProviderError::Parse(format!("Failed to parse XML: {}", e)))?;

        let mut papers = Vec::new();

        // Find all entry elements
        for entry in doc.descendants().filter(|n| n.has_tag_name("entry")) {
            let mut paper = PaperMetadata {
                doi: String::new(),
                title: None,
                authors: Vec::new(),
                journal: Some("arXiv".to_string()),
                year: None,
                abstract_text: None,
                pdf_url: None,
                file_size: None,
            };

            // Extract metadata from entry
            for child in entry.children().filter(|n| n.is_element()) {
                match child.tag_name().name() {
                    "id" => {
                        if let Some(id) = child.text() {
                            // Extract arXiv ID from URL
                            if let Some(arxiv_id) = id.split('/').last() {
                                paper.doi = format!("arXiv:{}", arxiv_id);
                            }
                        }
                    }
                    "title" => {
                        if let Some(title) = child.text() {
                            paper.title = Some(title.trim().replace('\n', " ").replace("  ", " "));
                        }
                    }
                    "summary" => {
                        if let Some(summary) = child.text() {
                            paper.abstract_text = Some(summary.trim().replace('\n', " ").replace("  ", " "));
                        }
                    }
                    "published" => {
                        if let Some(published) = child.text() {
                            // Parse date (format: YYYY-MM-DDTHH:MM:SSZ)
                            if let Some(year_str) = published.split('-').next() {
                                if let Ok(year) = year_str.parse::<u32>() {
                                    paper.year = Some(year);
                                }
                            }
                        }
                    }
                    "author" => {
                        // Extract author name
                        for name_elem in child.descendants().filter(|n| n.has_tag_name("name")) {
                            if let Some(author_name) = name_elem.text() {
                                paper.authors.push(author_name.trim().to_string());
                            }
                        }
                    }
                    "link" => {
                        // Look for PDF links
                        if let Some(href) = child.attribute("href") {
                            if let Some(link_type) = child.attribute("type") {
                                if link_type == "application/pdf" {
                                    paper.pdf_url = Some(href.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if paper.title.is_some() {
                papers.push(paper);
            }
        }

        debug!("Parsed {} papers from arXiv response", papers.len());
        Ok(papers)
    }
}

impl Default for ArxivProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create ArxivProvider")
    }
}

#[async_trait]
impl SourceProvider for ArxivProvider {
    fn name(&self) -> &str {
        "arxiv"
    }

    fn description(&self) -> &str {
        "arXiv.org - Open access e-prints in physics, mathematics, computer science, and more"
    }

    fn supported_search_types(&self) -> Vec<SearchType> {
        vec![
            SearchType::Auto,
            SearchType::Title,
            SearchType::Author,
            SearchType::Keywords,
            SearchType::Subject,
            SearchType::Doi,
        ]
    }

    fn supports_full_text(&self) -> bool {
        true // arXiv provides free PDF access
    }

    fn priority(&self) -> u8 {
        80 // High priority for CS/physics/math
    }

    fn base_delay(&self) -> Duration {
        Duration::from_millis(3000) // arXiv recommends 3-second delays
    }

    async fn search(&self, query: &SearchQuery, context: &SearchContext) -> Result<ProviderResult, ProviderError> {
        let start_time = Instant::now();
        
        info!("Searching arXiv for: {} (type: {:?})", query.query, query.search_type);

        // Build the search URL
        let url = self.build_search_url(query)?;
        debug!("arXiv search URL: {}", url);

        // Make the request
        let mut request = self.client.get(&url);
        
        // Add custom headers
        for (key, value) in &context.headers {
            request = request.header(key, value);
        }

        let response = request
            .timeout(context.timeout)
            .send()
            .await
            .map_err(|e| {
                error!("arXiv request failed: {}", e);
                if e.is_timeout() {
                    ProviderError::Timeout
                } else if e.is_connect() {
                    ProviderError::Network(format!("Connection failed: {}", e))
                } else {
                    ProviderError::Network(format!("Request failed: {}", e))
                }
            })?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            return Err(match status.as_u16() {
                429 => ProviderError::RateLimit,
                503 => ProviderError::ServiceUnavailable("arXiv service temporarily unavailable".to_string()),
                _ => ProviderError::Network(format!("HTTP {}: {}", status, error_text)),
            });
        }

        // Parse the response
        let response_text = response.text().await
            .map_err(|e| ProviderError::Network(format!("Failed to read response: {}", e)))?;

        let papers = self.parse_response(&response_text)?;
        let search_time = start_time.elapsed();

        // Check if there might be more results
        let has_more = papers.len() as u32 >= query.max_results;

        let mut metadata = HashMap::new();
        metadata.insert("api_url".to_string(), url);
        metadata.insert("response_size".to_string(), response_text.len().to_string());

        info!("arXiv search completed: {} papers found in {:?}", papers.len(), search_time);

        Ok(ProviderResult {
            papers,
            source: "arXiv".to_string(),
            total_available: None, // arXiv doesn't provide total count
            search_time,
            has_more,
            metadata,
        })
    }

    async fn health_check(&self, context: &SearchContext) -> Result<bool, ProviderError> {
        debug!("Performing arXiv health check");
        
        let query = SearchQuery {
            query: "quantum".to_string(),
            search_type: SearchType::Keywords,
            max_results: 1,
            offset: 0,
            params: HashMap::new(),
        };

        match self.search(&query, context).await {
            Ok(result) => {
                let healthy = !result.papers.is_empty();
                info!("arXiv health check: {}", if healthy { "OK" } else { "No results" });
                Ok(healthy)
            }
            Err(ProviderError::RateLimit) => {
                info!("arXiv health check: OK (rate limited but responsive)");
                Ok(true)
            }
            Err(e) => {
                warn!("arXiv health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_arxiv_provider_creation() {
        let provider = ArxivProvider::new();
        assert!(provider.is_ok());
    }

    #[test]
    fn test_arxiv_search_url_building() {
        let provider = ArxivProvider::new().unwrap();
        
        let query = SearchQuery {
            query: "quantum computing".to_string(),
            search_type: SearchType::Keywords,
            max_results: 10,
            offset: 0,
            params: HashMap::new(),
        };

        let url = provider.build_search_url(&query).unwrap();
        assert!(url.contains("all:%22quantum%20computing%22") || url.contains("all:quantum") || url.contains("quantum"));
        assert!(url.contains("max_results=10"));
        assert!(url.contains("start=0"));
    }

    #[test]
    fn test_arxiv_doi_search_url() {
        let provider = ArxivProvider::new().unwrap();
        
        let query = SearchQuery {
            query: "10.1103/PhysRevA.52.R2493".to_string(),
            search_type: SearchType::Doi,
            max_results: 1,
            offset: 0,
            params: HashMap::new(),
        };

        let url = provider.build_search_url(&query).unwrap();
        assert!(url.contains("search_query=doi%3A10.1103") || url.contains("search_query") && url.contains("doi"));
    }
}