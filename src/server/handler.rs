use crate::{Config, Result, SciHubClient, SearchTool, DownloadTool, MetadataExtractor};
use std::sync::Arc;
use tracing::{debug, info, instrument};

#[derive(Debug)]
pub struct SciHubServerHandler {
    config: Arc<Config>,
    #[allow(dead_code)] // Will be used when MCP integration is complete
    search_tool: SearchTool,
    #[allow(dead_code)] // Will be used when MCP integration is complete
    download_tool: DownloadTool,
    #[allow(dead_code)] // Will be used when MCP integration is complete
    metadata_extractor: MetadataExtractor,
}

impl SciHubServerHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        info!("Initializing SciHub MCP server handler");
        
        // Initialize Sci-Hub client
        let client = Arc::new(SciHubClient::new((*config).clone())?);
        
        // Initialize search tool
        let search_tool = SearchTool::new(client.clone(), config.clone());
        
        // Initialize download tool
        let download_tool = DownloadTool::new(client, config.clone());
        
        // Initialize metadata extractor
        let metadata_extractor = MetadataExtractor::new(config.clone())?;
        
        Ok(Self { config, search_tool, download_tool, metadata_extractor })
    }
}

// For now, create a simple health check function
// Full MCP handler implementation will be done when we have the correct rmcp API
impl SciHubServerHandler {
    #[instrument(skip(self))]
    pub async fn ping(&self) -> Result<()> {
        debug!("Ping received - server is healthy");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> Result<String> {
        info!("MCP server initializing");
        debug!("Server configuration loaded with {} mirrors", self.config.sci_hub.mirrors.len());
        Ok("rust-sci-hub-mcp".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_handler() -> SciHubServerHandler {
        let config = Config::default();
        SciHubServerHandler::new(Arc::new(config)).unwrap()
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let handler = create_test_handler();
        assert!(handler.config.sci_hub.mirrors.len() > 0);
    }

    #[tokio::test]
    async fn test_handler_initialization() {
        let mut handler = create_test_handler();
        let result = handler.initialize().await;
        assert!(result.is_ok());
        
        let server_name = result.unwrap();
        assert_eq!(server_name, "rust-sci-hub-mcp");
    }

    #[tokio::test]
    async fn test_ping() {
        let handler = create_test_handler();
        let result = handler.ping().await;
        assert!(result.is_ok());
    }
}