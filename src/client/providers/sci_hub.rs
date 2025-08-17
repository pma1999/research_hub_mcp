use super::traits::{
    ProviderError, ProviderResult, SearchContext, SearchQuery, SearchType, SourceProvider,
};
use crate::client::{PaperMetadata, ResearchClient, ResearchResponse};
use crate::{Config, Doi};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Sci-Hub provider adapter
pub struct SciHubProvider {
    client: Arc<ResearchClient>,
}

impl SciHubProvider {
    /// Create a new Sci-Hub provider
    pub fn new(config: Config) -> Result<Self, ProviderError> {
        let client = ResearchClient::new(config)
            .map_err(|e| ProviderError::Other(format!("Failed to create Sci-Hub client: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Convert search query to Sci-Hub format
    fn convert_query(&self, query: &SearchQuery) -> String {
        match query.search_type {
            SearchType::Doi => {
                // Clean DOI format
                query
                    .query
                    .trim_start_matches("doi:")
                    .trim_start_matches("https://doi.org/")
                    .to_string()
            }
            _ => query.query.clone(),
        }
    }

    /// Convert ResearchResponse to ProviderResult
    fn convert_response(
        &self,
        response: ResearchResponse,
        start_time: Instant,
        query: &SearchQuery,
    ) -> ProviderResult {
        let papers = if response.found {
            vec![response.metadata]
        } else {
            Vec::new()
        };

        let search_time = start_time.elapsed();

        let mut metadata = HashMap::new();
        metadata.insert(
            "source_mirror".to_string(),
            response.source_mirror.to_string(),
        );
        metadata.insert(
            "response_time".to_string(),
            response.response_time.as_millis().to_string(),
        );

        ProviderResult {
            papers,
            source: "Sci-Hub".to_string(),
            total_available: if response.found { Some(1) } else { Some(0) },
            search_time,
            has_more: false, // Sci-Hub typically returns single results
            metadata,
        }
    }
}

#[async_trait]
impl SourceProvider for SciHubProvider {
    fn name(&self) -> &str {
        "sci_hub"
    }

    fn description(&self) -> &str {
        "Sci-Hub - Free access to academic papers"
    }

    fn supported_search_types(&self) -> Vec<SearchType> {
        vec![SearchType::Auto, SearchType::Doi, SearchType::Title]
    }

    fn supports_full_text(&self) -> bool {
        true // Sci-Hub provides PDF access
    }

    fn priority(&self) -> u8 {
        30 // Lower priority, use as fallback for full-text
    }

    fn base_delay(&self) -> Duration {
        Duration::from_secs(2) // Respectful delay for Sci-Hub
    }

    async fn search(
        &self,
        query: &SearchQuery,
        _context: &SearchContext,
    ) -> Result<ProviderResult, ProviderError> {
        let start_time = Instant::now();

        info!(
            "Searching Sci-Hub for: {} (type: {:?})",
            query.query, query.search_type
        );

        // Convert query to Sci-Hub format
        let search_term = self.convert_query(query);

        // Try to search based on query type
        let response = match query.search_type {
            SearchType::Doi => {
                // For DOI searches, use the DOI search method
                let doi = Doi::new(&search_term)
                    .map_err(|e| ProviderError::InvalidQuery(format!("Invalid DOI: {}", e)))?;

                self.client.search_by_doi(&doi).await.map_err(|e| {
                    ProviderError::Other(format!("Sci-Hub DOI search failed: {}", e))
                })?
            }
            SearchType::Title | SearchType::Auto | SearchType::Keywords => {
                // For title/auto searches, use the title search method
                self.client
                    .search_by_title(&search_term)
                    .await
                    .map_err(|e| {
                        ProviderError::Other(format!("Sci-Hub title search failed: {}", e))
                    })?
            }
            SearchType::Author => {
                return Err(ProviderError::InvalidQuery(
                    "Sci-Hub does not support author search".to_string(),
                ));
            }
            SearchType::Subject => {
                return Err(ProviderError::InvalidQuery(
                    "Sci-Hub does not support subject search".to_string(),
                ));
            }
        };

        let result = self.convert_response(response, start_time, query);

        info!(
            "Sci-Hub search completed: {} papers found in {:?}",
            result.papers.len(),
            result.search_time
        );

        Ok(result)
    }

    async fn get_by_doi(
        &self,
        doi: &str,
        _context: &SearchContext,
    ) -> Result<Option<PaperMetadata>, ProviderError> {
        info!("Getting paper by DOI from Sci-Hub: {}", doi);

        let doi_obj = Doi::new(doi)
            .map_err(|e| ProviderError::InvalidQuery(format!("Invalid DOI: {}", e)))?;

        let response = self
            .client
            .search_by_doi(&doi_obj)
            .await
            .map_err(|e| ProviderError::Other(format!("Sci-Hub DOI lookup failed: {}", e)))?;

        if response.found {
            Ok(Some(response.metadata))
        } else {
            Ok(None)
        }
    }

    async fn health_check(&self, _context: &SearchContext) -> Result<bool, ProviderError> {
        debug!("Performing Sci-Hub health check");

        // Try a simple DOI lookup
        let test_doi = "10.1038/nature12373";

        match self.get_by_doi(test_doi, _context).await {
            Ok(_) => {
                info!("Sci-Hub health check: OK");
                Ok(true)
            }
            Err(ProviderError::RateLimit) => {
                info!("Sci-Hub health check: OK (rate limited but responsive)");
                Ok(true)
            }
            Err(e) => {
                warn!("Sci-Hub health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_sci_hub_provider_creation() {
        let config = Config::default();
        let provider = SciHubProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_query_conversion() {
        let config = Config::default();
        let provider = SciHubProvider::new(config).unwrap();

        let query = SearchQuery {
            query: "doi:10.1038/nature12373".to_string(),
            search_type: SearchType::Doi,
            max_results: 1,
            offset: 0,
            params: HashMap::new(),
        };

        let converted = provider.convert_query(&query);
        assert_eq!(converted, "10.1038/nature12373");
    }
}
