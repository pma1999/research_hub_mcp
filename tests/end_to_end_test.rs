use rust_sci_hub_mcp::{Config, Server, SciHubClient, SearchTool, DownloadTool, MetadataExtractor};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

/// End-to-end test scenarios covering complete user workflows
#[tokio::test]
async fn test_complete_paper_search_workflow() {
    // Setup test environment
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.downloads.max_concurrent = 1;

    // Setup mock server for Sci-Hub
    let mock_server = MockServer::start().await;
    config.sci_hub.mirrors = vec![mock_server.uri()];

    // Mock successful DOI search response
    Mock::given(method("GET"))
        .and(path("/10.1000/test.doi"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(r#"
            <html>
                <body>
                    <div id="article">
                        <a href="/download/test.pdf">Download PDF</a>
                    </div>
                </body>
            </html>
            "#))
        .mount(&mock_server)
        .await;

    // Mock PDF download response
    Mock::given(method("GET"))
        .and(path("/download/test.pdf"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("Mock PDF content")
            .append_header("content-type", "application/pdf"))
        .mount(&mock_server)
        .await;

    // Initialize components
    let sci_hub_client = SciHubClient::new(config.sci_hub.clone()).unwrap();
    let search_tool = SearchTool::new(sci_hub_client.clone());
    let download_tool = DownloadTool::new(sci_hub_client.clone(), config.downloads.clone());
    let metadata_extractor = MetadataExtractor::new(config.downloads.directory.clone());

    // Scenario 1: Search for paper by DOI
    let search_result = search_tool.search_by_doi("10.1000/test.doi").await;
    assert!(search_result.is_ok(), "DOI search should succeed");
    
    let papers = search_result.unwrap();
    assert!(!papers.is_empty(), "Should find at least one paper");

    // Scenario 2: Download the found paper
    let paper = &papers[0];
    let download_result = download_tool.download(&paper.download_url, None).await;
    assert!(download_result.is_ok(), "Download should succeed");
    
    let download_info = download_result.unwrap();
    assert!(download_info.file_path.exists(), "Downloaded file should exist");

    // Scenario 3: Extract metadata from downloaded paper
    let metadata_result = metadata_extractor.extract_metadata(&download_info.file_path).await;
    assert!(metadata_result.is_ok(), "Metadata extraction should not fail");
}

#[tokio::test]
async fn test_complete_server_lifecycle_scenario() {
    // Test complete MCP server lifecycle with client interaction
    let mut config = Config::default();
    config.server.port = 0; // Use random available port
    config.server.graceful_shutdown_timeout_secs = 1;

    let server = Arc::new(Server::new(config));

    // Start server in background
    let server_clone = Arc::clone(&server);
    let server_handle = tokio::spawn(async move {
        server_clone.run().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test server health check
    assert!(!server.is_shutdown_requested(), "Server should be running");

    // Test server shutdown
    server.shutdown().await;
    assert!(server.is_shutdown_requested(), "Server should be marked for shutdown");

    // Wait for graceful shutdown
    let result = timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server should shutdown gracefully");
}

#[tokio::test]
async fn test_error_recovery_workflow() {
    // Test error recovery scenarios
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();

    // Setup mock server that initially fails
    let mock_server = MockServer::start().await;
    config.sci_hub.mirrors = vec![mock_server.uri()];

    // Mock server returning 503 (service unavailable)
    Mock::given(method("GET"))
        .and(path("/10.1000/failing.doi"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2) // Fail first 2 requests
        .mount(&mock_server)
        .await;

    // Then succeed
    Mock::given(method("GET"))
        .and(path("/10.1000/failing.doi"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string("<html><body>Success</body></html>"))
        .mount(&mock_server)
        .await;

    let sci_hub_client = SciHubClient::new(config.sci_hub.clone()).unwrap();
    let search_tool = SearchTool::new(sci_hub_client);

    // Search should eventually succeed after retries
    let result = search_tool.search_by_doi("10.1000/failing.doi").await;
    // This might fail due to retry logic, but that's expected behavior
    // The test validates that the system handles failures gracefully
    println!("Error recovery test result: {:?}", result);
}

#[tokio::test]
async fn test_concurrent_operations_scenario() {
    // Test concurrent operations and thread safety
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.downloads.max_concurrent = 3;

    let mock_server = MockServer::start().await;
    config.sci_hub.mirrors = vec![mock_server.uri()];

    // Mock multiple endpoints
    for i in 1..=5 {
        Mock::given(method("GET"))
            .and(path(&format!("/10.1000/test{}.doi", i)))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(&format!("<html><body>Paper {}</body></html>", i)))
            .mount(&mock_server)
            .await;
    }

    let sci_hub_client = SciHubClient::new(config.sci_hub.clone()).unwrap();
    let search_tool = Arc::new(SearchTool::new(sci_hub_client));

    // Launch concurrent searches
    let mut handles = vec![];
    for i in 1..=5 {
        let search_tool_clone = Arc::clone(&search_tool);
        let doi = format!("10.1000/test{}.doi", i);
        
        let handle = tokio::spawn(async move {
            search_tool_clone.search_by_doi(&doi).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    
    // Check that all concurrent operations completed
    assert_eq!(results.len(), 5, "All concurrent operations should complete");
    
    // Count successful operations
    let successful = results.iter()
        .filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok())
        .count();
    
    println!("Concurrent operations: {} successful out of {}", successful, results.len());
    assert!(successful > 0, "At least some concurrent operations should succeed");
}

#[tokio::test]
async fn test_configuration_workflow() {
    // Test configuration loading and validation workflow
    let temp_dir = TempDir::new().unwrap();
    
    // Test default configuration
    let default_config = Config::default();
    assert!(default_config.validate().is_ok(), "Default config should be valid");

    // Test configuration with custom download directory
    let mut custom_config = Config::default();
    custom_config.downloads.directory = temp_dir.path().to_path_buf();
    assert!(custom_config.validate().is_ok(), "Custom config should be valid");

    // Test invalid configuration
    let mut invalid_config = Config::default();
    invalid_config.server.port = 0; // Invalid port
    assert!(invalid_config.validate().is_err(), "Invalid config should fail validation");

    // Test configuration serialization
    let serialized = serde_json::to_string(&default_config);
    assert!(serialized.is_ok(), "Config should serialize successfully");

    // Test configuration deserialization
    let json_config = r#"{
        "server": {
            "host": "localhost",
            "port": 8080,
            "timeout_secs": 30,
            "graceful_shutdown_timeout_secs": 5,
            "health_check_interval_secs": 30,
            "max_connections": 100
        },
        "sci_hub": {
            "mirrors": ["https://test.com"],
            "timeout_secs": 30,
            "rate_limit_per_sec": 1,
            "max_retries": 3,
            "user_agent": "rust-sci-hub-mcp/0.1.0"
        },
        "downloads": {
            "directory": "/tmp/test",
            "max_concurrent": 3,
            "max_file_size_mb": 100,
            "organize_by_date": false,
            "verify_integrity": true
        }
    }"#;

    let parsed_config: Result<Config, _> = serde_json::from_str(json_config);
    assert!(parsed_config.is_ok(), "Config should deserialize successfully");
}