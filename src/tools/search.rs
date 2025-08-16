use crate::client::{Doi, PaperMetadata, SciHubClient, SciHubResponse};
use crate::{Config, Result};
// use rmcp::tool; // Will be enabled when rmcp integration is complete
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// Input parameters for the paper search tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchInput {
    /// Query string - can be DOI, title, or author name
    pub query: String,
    /// Type of search to perform
    #[serde(default)]
    pub search_type: SearchType,
    /// Maximum number of results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: u32,
}

/// Type of search to perform
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    /// Automatic detection based on query format
    #[default]
    Auto,
    /// Search by DOI
    Doi,
    /// Search by paper title
    Title,
    /// Search by author name
    Author,
    /// Search by combination of author and year
    AuthorYear,
}

/// Result of a paper search operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    /// Search query that was executed
    pub query: String,
    /// Type of search that was performed
    pub search_type: SearchType,
    /// List of papers found
    pub papers: Vec<PaperResult>,
    /// Total number of results available
    pub total_count: u32,
    /// Number of results returned in this response
    pub returned_count: u32,
    /// Offset used for this search
    pub offset: u32,
    /// Whether there are more results available
    pub has_more: bool,
    /// Time taken to execute the search in milliseconds
    pub search_time_ms: u64,
    /// Source mirror that provided the results
    pub source_mirror: Option<String>,
}

/// Individual paper result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaperResult {
    /// Paper metadata
    #[serde(flatten)]
    pub metadata: PaperMetadata,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f64,
    /// Whether the full paper is available for download
    pub available: bool,
    /// Source where this result came from
    pub source: String,
}

/// Cache entry for search results
#[derive(Debug, Clone)]
struct CacheEntry {
    result: SearchResult,
    timestamp: SystemTime,
    ttl: Duration,
}

impl CacheEntry {
    fn new(result: SearchResult, ttl: Duration) -> Self {
        Self {
            result,
            timestamp: SystemTime::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.timestamp.elapsed().unwrap_or(Duration::MAX) > self.ttl
    }
}

/// Paper search tool implementation
#[derive(Clone)]
pub struct SearchTool {
    client: Arc<SciHubClient>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    config: Arc<Config>,
}

impl std::fmt::Debug for SearchTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchTool")
            .field("client", &"SciHubClient")
            .field("cache", &"RwLock<HashMap>")
            .field("config", &"Config")
            .finish()
    }
}

impl SearchTool {
    /// Create a new search tool
    pub fn new(client: Arc<SciHubClient>, config: Arc<Config>) -> Self {
        info!("Initializing paper search tool");
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Execute a paper search
    // #[tool] // Will be enabled when rmcp integration is complete
    #[instrument(skip(self), fields(query = %input.query, search_type = ?input.search_type))]
    pub async fn search_papers(&self, input: SearchInput) -> Result<SearchResult> {
        info!("Executing paper search: query='{}', type={:?}", input.query, input.search_type);
        
        // Validate input
        Self::validate_input(&input)?;
        
        // Check cache first
        let cache_key = Self::generate_cache_key(&input);
        if let Some(cached_result) = self.get_from_cache(&cache_key).await {
            debug!("Returning cached search result for query: {}", input.query);
            return Ok(cached_result);
        }

        let start_time = SystemTime::now();
        
        // Determine actual search type
        let search_type = Self::determine_search_type(&input.query, &input.search_type);
        debug!("Determined search type: {:?}", search_type);

        // Execute search based on type
        let sci_hub_response = match search_type {
            SearchType::Doi => {
                self.search_by_doi(&input.query).await?
            }
            SearchType::Auto if Self::looks_like_doi(&input.query) => {
                self.search_by_doi(&input.query).await?
            }
            SearchType::Title | SearchType::Auto => {
                self.search_by_title(&input.query).await?
            }
            SearchType::Author => {
                return Err(crate::Error::InvalidInput {
                    field: "search_type".to_string(),
                    reason: "Author search not yet implemented - use title search instead".to_string(),
                });
            }
            SearchType::AuthorYear => {
                return Err(crate::Error::InvalidInput {
                    field: "search_type".to_string(),
                    reason: "Author+year search not yet implemented - use title search instead".to_string(),
                });
            }
        };

        let search_time = start_time.elapsed().unwrap_or(Duration::ZERO);
        
        // Convert to search result
        let result = Self::convert_to_search_result(
            input.query.clone(),
            search_type,
            sci_hub_response,
            &input,
            search_time,
        );

        // Cache the result
        self.cache_result(&cache_key, &result).await;

        info!("Search completed in {}ms, found {} results", 
              search_time.as_millis(), result.returned_count);
        
        Ok(result)
    }

    /// Validate search input parameters
    fn validate_input(input: &SearchInput) -> Result<()> {
        if input.query.trim().is_empty() {
            return Err(crate::Error::InvalidInput {
                field: "query".to_string(),
                reason: "Query cannot be empty".to_string(),
            });
        }

        if input.query.len() > 1000 {
            return Err(crate::Error::InvalidInput {
                field: "query".to_string(),
                reason: "Query too long (max 1000 characters)".to_string(),
            });
        }

        if input.limit == 0 || input.limit > 100 {
            return Err(crate::Error::InvalidInput {
                field: "limit".to_string(),
                reason: "Limit must be between 1 and 100".to_string(),
            });
        }

        // Basic sanitization - reject potentially malicious input
        if input.query.contains('\0') || input.query.contains('\x1b') {
            return Err(crate::Error::InvalidInput {
                field: "query".to_string(),
                reason: "Query contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Determine the appropriate search type
    fn determine_search_type(query: &str, requested_type: &SearchType) -> SearchType {
        match requested_type {
            SearchType::Auto => {
                if Self::looks_like_doi(query) {
                    SearchType::Doi
                } else {
                    SearchType::Title
                }
            }
            other => other.clone(),
        }
    }

    /// Check if a query looks like a DOI
    fn looks_like_doi(query: &str) -> bool {
        // Basic DOI pattern matching
        let cleaned = query.trim().trim_start_matches("doi:").trim_start_matches("https://doi.org/");
        cleaned.contains('/') && (
            cleaned.starts_with("10.") ||
            query.starts_with("doi:") ||
            query.starts_with("https://doi.org/")
        )
    }

    /// Search by DOI
    async fn search_by_doi(&self, query: &str) -> Result<SciHubResponse> {
        let doi = Doi::new(query)?;
        self.client.search_by_doi(&doi).await
    }

    /// Search by title
    async fn search_by_title(&self, query: &str) -> Result<SciHubResponse> {
        self.client.search_by_title(query).await
    }

    /// Convert `SciHubResponse` to `SearchResult`
    fn convert_to_search_result(
        original_query: String,
        search_type: SearchType,
        response: SciHubResponse,
        input: &SearchInput,
        search_time: Duration,
    ) -> SearchResult {
        let paper_result = PaperResult {
            metadata: response.metadata,
            relevance_score: if response.found { 1.0 } else { 0.0 },
            available: response.found,
            source: "Sci-Hub".to_string(),
        };

        let papers = if response.found { vec![paper_result] } else { vec![] };
        let returned_count = u32::try_from(papers.len()).unwrap_or(u32::MAX);

        SearchResult {
            query: original_query,
            search_type,
            papers,
            total_count: returned_count, // For now, we only get single results from Sci-Hub
            returned_count,
            offset: input.offset,
            has_more: false, // Sci-Hub typically returns single results
            search_time_ms: u64::try_from(search_time.as_millis()).unwrap_or(u64::MAX),
            source_mirror: Some(response.source_mirror.to_string()),
        }
    }

    /// Generate cache key for search input
    fn generate_cache_key(input: &SearchInput) -> String {
        format!("{}:{}:{}:{}", 
                input.query.to_lowercase(), 
                serde_json::to_string(&input.search_type).unwrap_or_default(),
                input.limit,
                input.offset)
    }

    /// Get result from cache
    async fn get_from_cache(&self, cache_key: &str) -> Option<SearchResult> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(cache_key) {
            if !entry.is_expired() {
                let result = entry.result.clone();
                drop(cache);
                return Some(result);
            }
        }
        drop(cache);
        None
    }

    /// Cache search result
    async fn cache_result(&self, cache_key: &str, result: &SearchResult) {
        let mut cache = self.cache.write().await;
        let ttl = Duration::from_secs(self.config.sci_hub.timeout_secs * 10); // Cache for 10x timeout
        cache.insert(cache_key.to_string(), CacheEntry::new(result.clone(), ttl));
        
        // Simple cache cleanup - remove expired entries
        cache.retain(|_, entry| !entry.is_expired());
        
        debug!("Cached search result, cache size: {}", cache.len());
    }

    /// Clear cache (useful for testing)
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
        debug!("Search cache cleared");
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|entry| entry.is_expired()).count();
        drop(cache);
        (total, expired)
    }
}

/// Default limit for search results
const fn default_limit() -> u32 {
    10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::PaperMetadata;
    use crate::config::{Config, SciHubConfig};

    fn create_test_config() -> Arc<Config> {
        let mut config = Config::default();
        config.sci_hub = SciHubConfig {
            mirrors: vec!["https://sci-hub.se".to_string()],
            rate_limit_per_sec: 1,
            timeout_secs: 30,
            max_retries: 2,
        };
        Arc::new(config)
    }

    fn create_test_search_tool() -> Result<SearchTool> {
        let config = create_test_config();
        let client = Arc::new(SciHubClient::new((*config).clone())?);
        Ok(SearchTool::new(client, config))
    }

    #[test]
    fn test_search_input_validation() {
        // Empty query should fail
        let empty_input = SearchInput {
            query: "".to_string(),
            search_type: SearchType::Auto,
            limit: 10,
            offset: 0,
        };
        assert!(SearchTool::validate_input(&empty_input).is_err());

        // Too long query should fail
        let long_input = SearchInput {
            query: "a".repeat(1001),
            search_type: SearchType::Auto,
            limit: 10,
            offset: 0,
        };
        assert!(SearchTool::validate_input(&long_input).is_err());

        // Invalid limit should fail
        let invalid_limit = SearchInput {
            query: "test".to_string(),
            search_type: SearchType::Auto,
            limit: 0,
            offset: 0,
        };
        assert!(SearchTool::validate_input(&invalid_limit).is_err());

        // Valid input should pass
        let valid_input = SearchInput {
            query: "10.1038/nature12373".to_string(),
            search_type: SearchType::Auto,
            limit: 10,
            offset: 0,
        };
        assert!(SearchTool::validate_input(&valid_input).is_ok());
    }

    #[test]
    fn test_doi_detection() {
        assert!(SearchTool::looks_like_doi("10.1038/nature12373"));
        assert!(SearchTool::looks_like_doi("doi:10.1038/nature12373"));
        assert!(SearchTool::looks_like_doi("https://doi.org/10.1038/nature12373"));
        assert!(!SearchTool::looks_like_doi("Nature paper about genetics"));
        assert!(!SearchTool::looks_like_doi("Smith et al 2023"));
    }

    #[test]
    fn test_search_type_determination() {

        // Auto detection should work
        let doi_type = SearchTool::determine_search_type("10.1038/nature12373", &SearchType::Auto);
        assert!(matches!(doi_type, SearchType::Doi));

        let title_type = SearchTool::determine_search_type("Machine learning in biology", &SearchType::Auto);
        assert!(matches!(title_type, SearchType::Title));

        // Explicit types should be preserved
        let explicit_title = SearchTool::determine_search_type("10.1038/nature12373", &SearchType::Title);
        assert!(matches!(explicit_title, SearchType::Title));
    }

    #[test]
    fn test_cache_key_generation() {
        let input = SearchInput {
            query: "Test Query".to_string(),
            search_type: SearchType::Title,
            limit: 10,
            offset: 0,
        };

        let key1 = SearchTool::generate_cache_key(&input);
        let key2 = SearchTool::generate_cache_key(&input);
        assert_eq!(key1, key2);

        // Different queries should generate different keys
        let mut input2 = input.clone();
        input2.query = "Different Query".to_string();
        let key3 = SearchTool::generate_cache_key(&input2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_convert_to_search_result() {
        let metadata = PaperMetadata::new("10.1038/test".to_string());
        let response = SciHubResponse {
            metadata,
            source_mirror: url::Url::parse("https://sci-hub.se").unwrap(),
            response_time: Duration::from_millis(500),
            found: true,
        };

        let input = SearchInput {
            query: "test query".to_string(),
            search_type: SearchType::Title,
            limit: 10,
            offset: 0,
        };

        let result = SearchTool::convert_to_search_result(
            "test query".to_string(),
            SearchType::Title,
            response,
            &input,
            Duration::from_millis(100),
        );

        assert_eq!(result.query, "test query");
        assert!(matches!(result.search_type, SearchType::Title));
        assert_eq!(result.returned_count, 1);
        assert_eq!(result.search_time_ms, 100);
        assert!(!result.has_more);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let tool = create_test_search_tool().unwrap();

        let input = SearchInput {
            query: "test".to_string(),
            search_type: SearchType::Title,
            limit: 10,
            offset: 0,
        };

        let result = SearchResult {
            query: "test".to_string(),
            search_type: SearchType::Title,
            papers: vec![],
            total_count: 0,
            returned_count: 0,
            offset: 0,
            has_more: false,
            search_time_ms: 100,
            source_mirror: None,
        };

        let cache_key = SearchTool::generate_cache_key(&input);

        // Initially should be empty
        assert!(tool.get_from_cache(&cache_key).await.is_none());

        // After caching should be available
        tool.cache_result(&cache_key, &result).await;
        let cached = tool.get_from_cache(&cache_key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().query, "test");

        // Clear cache
        tool.clear_cache().await;
        assert!(tool.get_from_cache(&cache_key).await.is_none());
    }
}