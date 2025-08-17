use crate::client::providers::{
    ArxivProvider, CrossRefProvider, ProviderError, ProviderResult, SciHubProvider, SearchContext,
    SearchQuery, SearchType, SourceProvider,
};
use crate::client::PaperMetadata;
use crate::Config;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Configuration for meta-search behavior
#[derive(Debug, Clone)]
pub struct MetaSearchConfig {
    /// Maximum number of providers to query in parallel
    pub max_parallel_providers: usize,
    /// Timeout for each provider
    pub provider_timeout: Duration,
    /// Whether to continue searching if some providers fail
    pub continue_on_failure: bool,
    /// Whether to deduplicate results
    pub deduplicate_results: bool,
    /// Minimum relevance score to include results
    pub min_relevance_score: f64,
}

impl Default for MetaSearchConfig {
    fn default() -> Self {
        Self {
            max_parallel_providers: 3,
            provider_timeout: Duration::from_secs(30),
            continue_on_failure: true,
            deduplicate_results: true,
            min_relevance_score: 0.0,
        }
    }
}

/// Result from meta-search across multiple providers
#[derive(Debug, Clone)]
pub struct MetaSearchResult {
    /// All papers found across providers
    pub papers: Vec<PaperMetadata>,
    /// Results grouped by source
    pub by_source: HashMap<String, Vec<PaperMetadata>>,
    /// Total search time
    pub total_search_time: Duration,
    /// Number of providers that succeeded
    pub successful_providers: usize,
    /// Number of providers that failed
    pub failed_providers: usize,
    /// Errors from failed providers
    pub provider_errors: HashMap<String, String>,
    /// Metadata from all providers
    pub provider_metadata: HashMap<String, HashMap<String, String>>,
}

/// Client that performs meta-search across multiple academic sources
pub struct MetaSearchClient {
    providers: Vec<Arc<dyn SourceProvider>>,
    config: MetaSearchConfig,
    rate_limiters: Arc<RwLock<HashMap<String, Instant>>>,
}

impl MetaSearchClient {
    /// Create a new meta-search client
    pub fn new(app_config: Config, meta_config: MetaSearchConfig) -> Result<Self, ProviderError> {
        let mut providers: Vec<Arc<dyn SourceProvider>> = Vec::new();

        // Add arXiv provider (high priority for CS/physics/math)
        providers.push(Arc::new(ArxivProvider::new()?));

        // Add CrossRef provider (high priority for metadata)
        providers.push(Arc::new(CrossRefProvider::new(None)?)); // TODO: Get email from config

        // Add Sci-Hub provider (lower priority, for full-text access)
        providers.push(Arc::new(SciHubProvider::new(app_config)?));

        info!(
            "Initialized meta-search client with {} providers",
            providers.len()
        );

        Ok(Self {
            providers,
            config: meta_config,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get list of available providers
    pub fn providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }

    /// Perform health checks on all providers
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let context = self.create_search_context().await;
        let mut results = HashMap::new();

        for provider in &self.providers {
            let health = provider.health_check(&context).await.unwrap_or(false);
            results.insert(provider.name().to_string(), health);

            if health {
                info!("Provider {} is healthy", provider.name());
            } else {
                warn!("Provider {} is unhealthy", provider.name());
            }
        }

        results
    }

    /// Search across multiple providers
    pub async fn search(&self, query: &SearchQuery) -> Result<MetaSearchResult, ProviderError> {
        let start_time = Instant::now();
        info!(
            "Starting meta-search for: {} (type: {:?})",
            query.query, query.search_type
        );

        // Create search context
        let context = self.create_search_context().await;

        // Filter providers based on query type and supported features
        let suitable_providers = self.filter_providers_for_query(query).await;
        info!(
            "Using {} providers for search: {:?}",
            suitable_providers.len(),
            suitable_providers
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
        );

        // Search providers in parallel (with concurrency limit)
        let mut provider_results = Vec::new();
        let mut provider_errors = HashMap::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_parallel_providers,
        ));

        let mut tasks = Vec::new();
        for provider in suitable_providers {
            let provider = provider.clone();
            let query = query.clone();
            let context = context.clone();
            let semaphore = semaphore.clone();
            let timeout_duration = self.config.provider_timeout;

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                // Apply rate limiting
                if let Err(e) = Self::apply_rate_limit(&provider).await {
                    return (provider.name().to_string(), Err(e));
                }

                // Search with timeout
                let result = timeout(timeout_duration, provider.search(&query, &context)).await;

                match result {
                    Ok(Ok(provider_result)) => (provider.name().to_string(), Ok(provider_result)),
                    Ok(Err(e)) => (provider.name().to_string(), Err(e)),
                    Err(_) => (provider.name().to_string(), Err(ProviderError::Timeout)),
                }
            });

            tasks.push(task);
        }

        // Collect results
        for task in tasks {
            match task.await {
                Ok((provider_name, Ok(result))) => {
                    info!(
                        "Provider {} returned {} results",
                        provider_name,
                        result.papers.len()
                    );
                    provider_results.push((provider_name, result));
                }
                Ok((provider_name, Err(error))) => {
                    warn!("Provider {} failed: {}", provider_name, error);
                    provider_errors.insert(provider_name, error.to_string());
                }
                Err(e) => {
                    error!("Task failed: {}", e);
                }
            }
        }

        // Aggregate results
        let meta_result = self
            .aggregate_results(provider_results, provider_errors, start_time)
            .await;

        info!(
            "Meta-search completed: {} total papers from {} providers in {:?}",
            meta_result.papers.len(),
            meta_result.successful_providers,
            meta_result.total_search_time
        );

        Ok(meta_result)
    }

    /// Search for a paper by DOI across providers that support it
    pub async fn get_by_doi(&self, doi: &str) -> Result<Option<PaperMetadata>, ProviderError> {
        info!("Searching for DOI: {}", doi);

        let context = self.create_search_context().await;

        // Try providers in priority order
        let mut providers: Vec<_> = self.providers.iter().collect();
        providers.sort_by_key(|p| std::cmp::Reverse(p.priority()));

        for provider in providers {
            if provider.supported_search_types().contains(&SearchType::Doi) {
                // Apply rate limiting
                if let Err(e) = Self::apply_rate_limit(provider).await {
                    warn!("Rate limit hit for {}: {}", provider.name(), e);
                    continue;
                }

                match provider.get_by_doi(doi, &context).await {
                    Ok(Some(paper)) => {
                        info!("Found paper for DOI {} from {}", doi, provider.name());
                        return Ok(Some(paper));
                    }
                    Ok(None) => {
                        debug!("DOI {} not found in {}", doi, provider.name());
                    }
                    Err(e) => {
                        warn!("Error searching {} for DOI {}: {}", provider.name(), doi, e);
                    }
                }
            }
        }

        info!("DOI {} not found in any provider", doi);
        Ok(None)
    }

    /// Create search context with common settings
    async fn create_search_context(&self) -> SearchContext {
        SearchContext {
            timeout: self.config.provider_timeout,
            user_agent: "rust-research-mcp/0.2.0 (Academic Research Tool)".to_string(),
            rate_limit: Some(Duration::from_millis(1000)),
            headers: HashMap::new(),
        }
    }

    /// Filter providers based on query characteristics
    async fn filter_providers_for_query(
        &self,
        query: &SearchQuery,
    ) -> Vec<Arc<dyn SourceProvider>> {
        let mut suitable = Vec::new();

        for provider in &self.providers {
            // Check if provider supports the search type
            if provider
                .supported_search_types()
                .contains(&query.search_type)
                || provider
                    .supported_search_types()
                    .contains(&SearchType::Auto)
            {
                suitable.push(provider.clone());
            }
        }

        // Sort by priority (highest first)
        suitable.sort_by_key(|p| std::cmp::Reverse(p.priority()));

        suitable
    }

    /// Apply rate limiting for a provider
    async fn apply_rate_limit(provider: &Arc<dyn SourceProvider>) -> Result<(), ProviderError> {
        // Simple rate limiting - wait for base delay since last request
        let base_delay = provider.base_delay();

        // For now, just wait the base delay
        // In a more sophisticated implementation, we'd track per-provider timing
        tokio::time::sleep(base_delay).await;

        Ok(())
    }

    /// Aggregate results from multiple providers
    async fn aggregate_results(
        &self,
        provider_results: Vec<(String, ProviderResult)>,
        provider_errors: HashMap<String, String>,
        start_time: Instant,
    ) -> MetaSearchResult {
        let mut all_papers = Vec::new();
        let mut by_source = HashMap::new();
        let mut provider_metadata = HashMap::new();

        // Collect all papers and organize by source
        for (source, result) in provider_results.iter() {
            by_source.insert(source.clone(), result.papers.clone());
            provider_metadata.insert(source.clone(), result.metadata.clone());
            all_papers.extend(result.papers.clone());
        }

        // Deduplicate if requested
        if self.config.deduplicate_results {
            all_papers = self.deduplicate_papers(all_papers).await;
        }

        // Filter by relevance score if needed
        if self.config.min_relevance_score > 0.0 {
            // Note: We'd need to add relevance scoring to PaperMetadata
            // For now, include all papers
        }

        // Sort by source priority and then by some relevance metric
        // For now, just keep the order

        MetaSearchResult {
            papers: all_papers,
            by_source,
            total_search_time: start_time.elapsed(),
            successful_providers: provider_results.len(),
            failed_providers: provider_errors.len(),
            provider_errors,
            provider_metadata,
        }
    }

    /// Deduplicate papers based on DOI and title similarity
    async fn deduplicate_papers(&self, papers: Vec<PaperMetadata>) -> Vec<PaperMetadata> {
        let original_count = papers.len();
        let mut unique_papers = Vec::new();
        let mut seen_dois = HashSet::new();
        let mut seen_titles = HashSet::new();

        for paper in papers {
            let mut is_duplicate = false;

            // Check DOI duplicates
            if !paper.doi.is_empty() {
                if seen_dois.contains(&paper.doi) {
                    is_duplicate = true;
                } else {
                    seen_dois.insert(paper.doi.clone());
                }
            }

            // Check title duplicates (case-insensitive, normalized)
            if !is_duplicate {
                if let Some(title) = &paper.title {
                    let normalized_title = title.to_lowercase().replace(&[' ', '\t', '\n'], "");
                    if seen_titles.contains(&normalized_title) {
                        is_duplicate = true;
                    } else {
                        seen_titles.insert(normalized_title);
                    }
                }
            }

            if !is_duplicate {
                unique_papers.push(paper);
            }
        }

        debug!(
            "Deduplicated {} papers to {} unique papers",
            original_count,
            unique_papers.len()
        );

        unique_papers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_meta_search_client_creation() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_provider_listing() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config).unwrap();

        let providers = client.providers();
        assert!(providers.contains(&"arxiv".to_string()));
        assert!(providers.contains(&"crossref".to_string()));
        assert!(providers.contains(&"sci_hub".to_string()));
    }

    #[tokio::test]
    async fn test_deduplication() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config).unwrap();

        let papers = vec![
            PaperMetadata {
                doi: "10.1038/nature12373".to_string(),
                title: Some("Test Paper".to_string()),
                authors: vec!["Author 1".to_string()],
                journal: Some("Nature".to_string()),
                year: Some(2023),
                abstract_text: None,
                pdf_url: None,
                file_size: None,
            },
            PaperMetadata {
                doi: "10.1038/nature12373".to_string(), // Same DOI
                title: Some("Test Paper".to_string()),
                authors: vec!["Author 1".to_string()],
                journal: Some("Nature".to_string()),
                year: Some(2023),
                abstract_text: None,
                pdf_url: None,
                file_size: None,
            },
        ];

        let deduplicated = client.deduplicate_papers(papers).await;
        assert_eq!(deduplicated.len(), 1);
    }
}
