use crate::{Config, Result, SciHubClient, SearchTool, DownloadTool, MetadataExtractor};
use rmcp::{
    model::*,
    service::{RequestContext, RoleServer},
    ErrorData,
    ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{future::Future, sync::Arc};
use tracing::{debug, info, instrument};

// Tool input structures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchInput {
    /// Query string - can be DOI, title, or author name
    pub query: String,
    /// Maximum number of results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadInput {
    /// DOI or URL of the paper to download
    pub identifier: String,
    /// Optional output directory
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetadataInput {
    /// Path to the PDF file or DOI
    pub input: String,
}

/// Main MCP server handler implementing rmcp
#[derive(Debug, Clone)]
pub struct SciHubServerHandler {
    config: Arc<Config>,
    search_tool: SearchTool,
    download_tool: DownloadTool,
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
        
        Ok(Self { 
            config, 
            search_tool, 
            download_tool, 
            metadata_extractor,
        })
    }
    
    /// Health check for the server
    #[instrument(skip(self))]
    pub async fn ping(&self) -> Result<()> {
        debug!("Ping received - server is healthy");
        Ok(())
    }
}

impl ServerHandler for SciHubServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A Rust-based MCP server for Sci-Hub integration. Provides tools to search, download, and extract metadata from academic papers.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    #[instrument(skip(self, request, context))]
    fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<InitializeResult, ErrorData>> + Send + '_ {
        info!("MCP server initializing");
        
        async move {
            // Set peer info if not already set
            if context.peer.peer_info().is_none() {
                context.peer.set_peer_info(request);
            }
            
            Ok(InitializeResult {
                protocol_version: ProtocolVersion::default(),
                capabilities: ServerCapabilities::builder().enable_tools().build(),
                server_info: Implementation {
                    name: "rust-sci-hub-mcp".into(),
                    version: "0.1.0".into(),
                },
                instructions: Some("A Rust-based MCP server for Sci-Hub integration. Provides tools to search, download, and extract metadata from academic papers.".into()),
            })
        }
    }
}

/// Default limit for search results
const fn default_limit() -> u32 {
    10
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
    async fn test_ping() {
        let handler = create_test_handler();
        let result = handler.ping().await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_search_input_validation() {
        let input = SearchInput {
            query: "test".to_string(),
            limit: 10,
            offset: 0,
        };
        assert_eq!(input.query, "test");
        assert_eq!(input.limit, 10);
        assert_eq!(input.offset, 0);
    }
}