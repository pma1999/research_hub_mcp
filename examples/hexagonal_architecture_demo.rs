//! # Hexagonal Architecture Demonstration
//!
//! This example demonstrates how the hexagonal architecture (ports and adapters pattern)
//! is implemented in the rust-sci-hub-mcp project. It shows how tools (application core)
//! depend only on port interfaces, not concrete implementations, enabling easy testing
//! and flexibility in swapping implementations.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Application Core                         │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
//! │  │ SearchTool  │  │ DownloadTool│  │MetadataTool │            │
//! │  └─────────────┘  └─────────────┘  └─────────────┘            │
//! │           │              │                │                   │
//! └───────────┼──────────────┼────────────────┼───────────────────┘
//!             │              │                │
//!             ▼              ▼                ▼
//!       ┌───────────┐  ┌───────────┐  ┌───────────┐
//!       │   Port    │  │   Port    │  │   Port    │
//!       │Interface  │  │Interface  │  │Interface  │
//!       └───────────┘  └───────────┘  └───────────┘
//!             │              │                │
//!             ▼              ▼                ▼
//!       ┌───────────┐  ┌───────────┐  ┌───────────┐
//!       │  Adapter  │  │  Adapter  │  │  Adapter  │
//!       │(Real Impl)│  │(Real Impl)│  │(Real Impl)│
//!       └───────────┘  └───────────┘  └───────────┘
//!             │              │                │
//!             ▼              ▼                ▼
//!       ┌───────────┐  ┌───────────┐  ┌───────────┐
//!       │External   │  │External   │  │External   │
//!       │Services   │  │Services   │  │Services   │
//!       └───────────┘  └───────────┘  └───────────┘
//! ```
//!
//! ## Benefits Demonstrated
//!
//! 1. **Testability**: Easy to mock dependencies with test adapters
//! 2. **Flexibility**: Can swap implementations without changing business logic
//! 3. **Separation of Concerns**: Clear boundaries between core and infrastructure
//! 4. **Dependency Inversion**: High-level modules don't depend on low-level modules

use rust_sci_hub_mcp::{
    adapters::{MetaSearchAdapter, PaperDownloadAdapter, PdfMetadataAdapter},
    client::{MetaSearchClient, MetaSearchConfig},
    ports::{DownloadServicePort, MetadataServicePort, SearchServicePort},
    tools::{
        download::{DownloadInput, DownloadStatus},
        metadata::MetadataInput,
        search::{SearchInput, SearchType},
    },
    Config, Result,
};
use std::sync::Arc;
use tokio;

/// Mock search adapter for testing
#[derive(Debug, Clone)]
struct MockSearchAdapter {
    should_fail: bool,
}

impl MockSearchAdapter {
    fn new(should_fail: bool) -> Self {
        Self { should_fail }
    }
}

#[async_trait::async_trait]
impl SearchServicePort for MockSearchAdapter {
    async fn search_papers(
        &self,
        input: SearchInput,
    ) -> Result<rust_sci_hub_mcp::tools::search::SearchResult> {
        if self.should_fail {
            return Err(rust_sci_hub_mcp::Error::Service(
                "Mock search failure".to_string(),
            ));
        }

        // Return mock result
        Ok(rust_sci_hub_mcp::tools::search::SearchResult {
            query: input.query,
            search_type: input.search_type,
            papers: vec![], // Empty for mock
            total_count: 0,
            returned_count: 0,
            offset: input.offset,
            has_more: false,
            search_time_ms: 100,
            source_mirror: Some("Mock".to_string()),
            category: Some("mock_research".to_string()),
        })
    }

    async fn health_check(&self) -> Result<rust_sci_hub_mcp::ports::search_service::ServiceHealth> {
        Ok(rust_sci_hub_mcp::ports::search_service::ServiceHealth {
            status: rust_sci_hub_mcp::ports::search_service::HealthStatus::Healthy,
            providers: std::collections::HashMap::new(),
            checked_at: std::time::SystemTime::now(),
            uptime_seconds: 3600,
            error_rate_percent: 0.0,
        })
    }

    async fn get_metrics(&self) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("mock_searches".to_string(), serde_json::Value::from(42));
        Ok(metrics)
    }

    async fn clear_cache(&self) -> Result<()> {
        println!("Mock: Cache cleared");
        Ok(())
    }
}

/// Demonstrates dependency injection with ports
struct PaperResearchApplication {
    search_service: Arc<dyn SearchServicePort>,
    download_service: Option<Arc<dyn DownloadServicePort>>,
    metadata_service: Option<Arc<dyn MetadataServicePort>>,
}

impl PaperResearchApplication {
    fn new(search_service: Arc<dyn SearchServicePort>) -> Self {
        Self {
            search_service,
            download_service: None,
            metadata_service: None,
        }
    }

    fn with_download_service(mut self, download_service: Arc<dyn DownloadServicePort>) -> Self {
        self.download_service = Some(download_service);
        self
    }

    fn with_metadata_service(mut self, metadata_service: Arc<dyn MetadataServicePort>) -> Self {
        self.metadata_service = Some(metadata_service);
        self
    }

    /// Research workflow that depends only on port interfaces
    async fn research_paper(&self, query: &str) -> Result<ResearchResult> {
        println!("🔍 Starting research for: {}", query);

        // Step 1: Search for papers
        let search_input = SearchInput {
            query: query.to_string(),
            search_type: SearchType::Auto,
            limit: 5,
            offset: 0,
        };

        let search_result = self.search_service.search_papers(search_input).await?;
        println!(
            "✅ Found {} papers in {}ms",
            search_result.returned_count, search_result.search_time_ms
        );

        let mut downloaded_papers = Vec::new();
        let mut extracted_metadata = Vec::new();

        // Step 2: Download papers (if download service is available)
        if let Some(download_service) = &self.download_service {
            for paper in &search_result.papers.iter().take(2) {
                // Download first 2 papers
                if let Some(doi) = paper.metadata.doi.as_ref() {
                    let download_input = DownloadInput {
                        doi: Some(doi.clone()),
                        url: None,
                        filename: None,
                        directory: None,
                        category: search_result.category.clone(),
                        overwrite: false,
                        verify_integrity: true,
                    };

                    match download_service.download_paper(download_input).await {
                        Ok(download_result) => {
                            if matches!(download_result.status, DownloadStatus::Completed) {
                                println!("📄 Downloaded: {:?}", download_result.file_path);
                                downloaded_papers.push(download_result);
                            }
                        }
                        Err(e) => {
                            println!("⚠️  Download failed for {}: {}", doi, e);
                        }
                    }
                }
            }
        }

        // Step 3: Extract metadata (if metadata service is available)
        if let Some(metadata_service) = &self.metadata_service {
            for download_result in &downloaded_papers {
                if let Some(file_path) = &download_result.file_path {
                    let metadata_input = MetadataInput {
                        file_path: file_path.to_string_lossy().to_string(),
                        use_cache: true,
                        validate_external: false,
                        extract_references: false,
                        batch_files: None,
                    };

                    match metadata_service.extract_metadata(metadata_input).await {
                        Ok(metadata_result) => {
                            println!("📊 Extracted metadata from: {:?}", file_path);
                            extracted_metadata.push(metadata_result);
                        }
                        Err(e) => {
                            println!("⚠️  Metadata extraction failed: {}", e);
                        }
                    }
                }
            }
        }

        Ok(ResearchResult {
            search_result,
            downloaded_papers,
            extracted_metadata,
        })
    }

    /// Health check across all services
    async fn system_health_check(&self) -> Result<SystemHealth> {
        println!("🏥 Checking system health...");

        let search_health = self.search_service.health_check().await?;
        println!("  Search Service: {:?}", search_health.status);

        let download_health = if let Some(download_service) = &self.download_service {
            let health = download_service.health_check().await?;
            println!("  Download Service: {:?}", health.status);
            Some(health)
        } else {
            None
        };

        let metadata_health = if let Some(metadata_service) = &self.metadata_service {
            let health = metadata_service.health_check().await?;
            println!("  Metadata Service: {:?}", health.status);
            Some(health)
        } else {
            None
        };

        Ok(SystemHealth {
            search_health,
            download_health,
            metadata_health,
        })
    }
}

/// Result of the research workflow
#[derive(Debug)]
struct ResearchResult {
    search_result: rust_sci_hub_mcp::tools::search::SearchResult,
    downloaded_papers: Vec<rust_sci_hub_mcp::tools::download::DownloadResult>,
    extracted_metadata: Vec<rust_sci_hub_mcp::tools::metadata::MetadataResult>,
}

/// System health across all services
#[derive(Debug)]
struct SystemHealth {
    search_health: rust_sci_hub_mcp::ports::search_service::ServiceHealth,
    download_health: Option<rust_sci_hub_mcp::ports::download_service::DownloadServiceHealth>,
    metadata_health: Option<rust_sci_hub_mcp::ports::metadata_service::MetadataServiceHealth>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏗️  Hexagonal Architecture Demonstration");
    println!("=======================================");

    // Initialize configuration
    let config = Arc::new(Config::default());

    println!("\n📋 Scenario 1: Testing with Mock Adapter");
    println!("-----------------------------------------");

    // Create application with mock adapter (for testing)
    let mock_search = Arc::new(MockSearchAdapter::new(false));
    let app_with_mock = PaperResearchApplication::new(mock_search);

    // Test search with mock
    let mock_result = app_with_mock.research_paper("machine learning").await?;
    println!(
        "Mock search result: {} papers found",
        mock_result.search_result.returned_count
    );

    println!("\n📋 Scenario 2: Production Setup with Real Adapters");
    println!("--------------------------------------------------");

    // Create meta search client
    let meta_config = MetaSearchConfig::default();
    let meta_client = Arc::new(MetaSearchClient::new((*config).clone(), meta_config)?);

    // Create real adapters
    let search_adapter = Arc::new(MetaSearchAdapter::new(config.clone())?);
    let download_adapter = Arc::new(PaperDownloadAdapter::new(
        meta_client.clone(),
        config.clone(),
    )?);
    let metadata_adapter = Arc::new(PdfMetadataAdapter::new(config.clone())?);

    // Create application with real adapters
    let production_app = PaperResearchApplication::new(search_adapter)
        .with_download_service(download_adapter)
        .with_metadata_service(metadata_adapter);

    // Perform health check
    let _health = production_app.system_health_check().await?;

    println!("\n📋 Scenario 3: Demonstrating Adapter Swapping");
    println!("----------------------------------------------");

    // Create app with working mock first
    let working_mock = Arc::new(MockSearchAdapter::new(false));
    let app1 = PaperResearchApplication::new(working_mock);
    println!("App with working mock:");
    let _result1 = app1.research_paper("test query").await?;

    // Swap to failing mock to show error handling
    let failing_mock = Arc::new(MockSearchAdapter::new(true));
    let app2 = PaperResearchApplication::new(failing_mock);
    println!("App with failing mock:");
    match app2.research_paper("test query").await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected failure: {}", e),
    }

    println!("\n📋 Scenario 4: Service Metrics and Monitoring");
    println!("----------------------------------------------");

    // Demonstrate metrics collection through ports
    let metrics = app_with_mock.search_service.get_metrics().await?;
    println!("Search service metrics:");
    for (key, value) in metrics {
        println!("  {}: {}", key, value);
    }

    println!("\n✅ Hexagonal Architecture Benefits Demonstrated:");
    println!("   1. ✓ Easy testing with mock adapters");
    println!("   2. ✓ Flexible adapter swapping without code changes");
    println!("   3. ✓ Clear separation between business logic and infrastructure");
    println!("   4. ✓ Dependency inversion - core depends on abstractions");
    println!("   5. ✓ Uniform interface for monitoring and health checks");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_adapter_success() {
        let mock_adapter = Arc::new(MockSearchAdapter::new(false));
        let app = PaperResearchApplication::new(mock_adapter);

        let result = app.research_paper("test query").await;
        assert!(result.is_ok());

        let research_result = result.unwrap();
        assert_eq!(research_result.search_result.query, "test query");
    }

    #[tokio::test]
    async fn test_mock_adapter_failure() {
        let mock_adapter = Arc::new(MockSearchAdapter::new(true));
        let app = PaperResearchApplication::new(mock_adapter);

        let result = app.research_paper("test query").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let mock_adapter = Arc::new(MockSearchAdapter::new(false));
        let app = PaperResearchApplication::new(mock_adapter);

        let health = app.system_health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let mock_adapter = Arc::new(MockSearchAdapter::new(false));
        let metrics = mock_adapter.get_metrics().await.unwrap();
        assert!(metrics.contains_key("mock_searches"));
    }

    #[test]
    fn test_application_builder_pattern() {
        let mock_search = Arc::new(MockSearchAdapter::new(false));

        // Test that we can build the application with different service combinations
        let app1 = PaperResearchApplication::new(mock_search.clone());
        assert!(app1.download_service.is_none());
        assert!(app1.metadata_service.is_none());

        // This would work in a real scenario with actual adapters
        // let app2 = PaperResearchApplication::new(mock_search)
        //     .with_download_service(download_adapter)
        //     .with_metadata_service(metadata_adapter);
    }
}
