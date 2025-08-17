use crate::client::PaperMetadata;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Search query parameters for different providers
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Query string
    pub query: String,
    /// Search type hint
    pub search_type: SearchType,
    /// Maximum results to return
    pub max_results: u32,
    /// Search offset for pagination
    pub offset: u32,
    /// Additional provider-specific parameters
    pub params: HashMap<String, String>,
}

/// Type of search being performed
#[derive(Debug, Clone, PartialEq)]
pub enum SearchType {
    /// Automatic detection
    Auto,
    /// Search by DOI
    Doi,
    /// Search by title
    Title,
    /// Search by author
    Author,
    /// Search by keywords
    Keywords,
    /// Search by subject/category
    Subject,
}

/// Context for search operations
#[derive(Debug, Clone)]
pub struct SearchContext {
    /// Timeout for the search operation
    pub timeout: Duration,
    /// User agent string
    pub user_agent: String,
    /// Rate limit constraints
    pub rate_limit: Option<Duration>,
    /// Additional headers
    pub headers: HashMap<String, String>,
}

/// Result from a source provider
#[derive(Debug, Clone)]
pub struct ProviderResult {
    /// Papers found by the provider
    pub papers: Vec<PaperMetadata>,
    /// Source that provided the results
    pub source: String,
    /// Total number of results available (if known)
    pub total_available: Option<u32>,
    /// Time taken to execute the search
    pub search_time: Duration,
    /// Whether there are more results available
    pub has_more: bool,
    /// Provider-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Errors that can occur during provider operations
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Timeout occurred")]
    Timeout,

    #[error("Provider error: {0}")]
    Other(String),
}

/// Trait for academic source providers
#[async_trait]
pub trait SourceProvider: Send + Sync {
    /// Unique name/identifier for this provider
    fn name(&self) -> &str;

    /// Human-readable description of the provider
    fn description(&self) -> &str;

    /// Supported search types
    fn supported_search_types(&self) -> Vec<SearchType>;

    /// Whether this provider supports full-text access
    fn supports_full_text(&self) -> bool;

    /// Search for papers using this provider
    async fn search(
        &self,
        query: &SearchQuery,
        context: &SearchContext,
    ) -> Result<ProviderResult, ProviderError>;

    /// Get paper metadata by DOI (if supported)
    async fn get_by_doi(
        &self,
        doi: &str,
        context: &SearchContext,
    ) -> Result<Option<PaperMetadata>, ProviderError> {
        let query = SearchQuery {
            query: doi.to_string(),
            search_type: SearchType::Doi,
            max_results: 1,
            offset: 0,
            params: HashMap::new(),
        };

        let result = self.search(&query, context).await?;
        Ok(result.papers.into_iter().next())
    }

    /// Health check for the provider
    async fn health_check(&self, context: &SearchContext) -> Result<bool, ProviderError> {
        // Default implementation: try a simple search
        let query = SearchQuery {
            query: "test".to_string(),
            search_type: SearchType::Keywords,
            max_results: 1,
            offset: 0,
            params: HashMap::new(),
        };

        match self.search(&query, context).await {
            Ok(_) => Ok(true),
            Err(ProviderError::RateLimit) => Ok(true), // Rate limit means service is up
            Err(_) => Ok(false),
        }
    }

    /// Get the base delay between requests for rate limiting
    fn base_delay(&self) -> Duration {
        Duration::from_millis(1000) // Default 1 second
    }

    /// Priority of this provider (higher = more important)
    fn priority(&self) -> u8 {
        50 // Default medium priority
    }
}
