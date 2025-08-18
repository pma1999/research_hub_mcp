use crate::tools::{
    download::DownloadInput as ActualDownloadInput, metadata::MetadataInput as ActualMetadataInput,
    search::SearchInput as ActualSearchInput,
};
use crate::{Config, DownloadTool, MetaSearchClient, MetadataExtractor, Result, SearchTool};
use rmcp::{
    model::*,
    service::{RequestContext, RoleServer},
    ErrorData, ServerHandler,
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
pub struct ResearchServerHandler {
    config: Arc<Config>,
    search_tool: SearchTool,
    download_tool: DownloadTool,
    metadata_extractor: MetadataExtractor,
}

impl ResearchServerHandler {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        info!("Initializing Research MCP server handler");

        // Initialize MetaSearch client with default config
        let meta_config = crate::client::MetaSearchConfig::default();
        let client = Arc::new(MetaSearchClient::new((*config).clone(), meta_config)?);

        // Initialize search tool
        let search_tool = SearchTool::new(config.clone())?;

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

impl ServerHandler for ResearchServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("🔬 Research Hub MCP Server v0.2.1 - Academic paper search and retrieval.\n\nProvides tools to:\n• 🔍 Search across arXiv, CrossRef, and Sci-Hub\n• 📥 Download papers with configurable directories\n• 📊 Extract metadata from PDFs\n\nDesigned for personal academic research use only.".into()),
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
                    name: "rust-research-mcp".into(),
                    version: "0.2.0".into(),
                },
                instructions: Some("A Rust-based MCP server for academic research paper access. Provides tools to search, download, and extract metadata from academic papers.".into()),
            })
        }
    }

    #[instrument(skip(self, _request, _context))]
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, ErrorData>> + Send + '_ {
        info!("Listing available tools");

        async move {
            let tools = vec![
                Tool {
                    name: "debug_test".into(), 
                    description: Some("Simple test tool for debugging - just echoes back what it receives".into()),
                    input_schema: Arc::new(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "Test message to echo back"
                            }
                        },
                        "required": ["message"]
                    }).as_object().unwrap().clone()),
                    output_schema: None,
                    annotations: None,
                },
                Tool {
                    name: "search_papers".into(),
                    description: Some("Search for academic papers using DOI, title, or author name".into()),
                    input_schema: Arc::new(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Query string - can be DOI, title, or author name"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of results to return",
                                "default": 10,
                                "minimum": 1,
                                "maximum": 100
                            }
                        },
                        "required": ["query"]
                    }).as_object().unwrap().clone()),
                    output_schema: None,
                    annotations: None,
                },
                Tool {
                    name: "download_paper".into(), 
                    description: Some("Download a paper PDF by DOI. Papers are saved to the configured download directory.".into()),
                    input_schema: Arc::new(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "doi": {
                                "type": "string",
                                "description": "DOI of the paper to download (e.g., '10.1038/nature12373')"
                            },
                            "filename": {
                                "type": "string", 
                                "description": "Optional custom filename for the downloaded PDF"
                            }
                        },
                        "required": ["doi"]
                    }).as_object().unwrap().clone()),
                    output_schema: None,
                    annotations: None,
                },
                Tool {
                    name: "extract_metadata".into(),
                    description: Some("Extract bibliographic metadata from a PDF file or DOI".into()),
                    input_schema: Arc::new(serde_json::to_value(schemars::schema_for!(ActualMetadataInput)).unwrap().as_object().unwrap().clone()),
                    output_schema: None,
                    annotations: None,
                },
            ];

            Ok(ListToolsResult {
                tools,
                next_cursor: None,
            })
        }
    }

    #[instrument(skip(self, request, _context))]
    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, ErrorData>> + Send + '_ {
        info!("Tool called: {}", request.name);

        let search_tool = self.search_tool.clone();
        let download_tool = self.download_tool.clone();
        let metadata_extractor = self.metadata_extractor.clone();

        async move {
            match request.name.as_ref() {
                "debug_test" => {
                    info!("Debug tool called with arguments: {:?}", request.arguments);
                    let message = request
                        .arguments
                        .and_then(|args| {
                            args.get("message")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_else(|| "No message provided".to_string());

                    Ok(CallToolResult {
                        content: Some(vec![Content::text(format!("Debug echo: {}", message))]),
                        structured_content: None,
                        is_error: Some(false),
                    })
                }
                "search_papers" => {
                    // Simple parsing for simplified schema
                    let args = request.arguments.unwrap_or_default();
                    let query = args.get("query").and_then(|v| v.as_str()).ok_or_else(|| {
                        ErrorData::invalid_params(
                            "Missing required 'query' parameter".to_string(),
                            None,
                        )
                    })?;
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as u32;

                    let input = ActualSearchInput {
                        query: query.to_string(),
                        search_type: crate::tools::search::SearchType::Auto,
                        limit,
                        offset: 0,
                    };

                    let results = search_tool.search_papers(input).await.map_err(|e| {
                        ErrorData::internal_error(format!("Search failed: {}", e), None)
                    })?;

                    Ok(CallToolResult {
                        content: Some(vec![Content::text(format!("📚 Found {} papers for '{}'\n\n{}\n\n💡 Tip: Papers from {} may be available for download. Very recent papers (2024-2025) might not be available yet.", 
                            results.returned_count,
                            results.query,
                            results.papers.iter().enumerate().map(|(i, p)| {
                                let doi_info = if !p.metadata.doi.is_empty() {
                                    format!("\n  📖 DOI: {}", p.metadata.doi)
                                } else {
                                    format!("\n  ⚠️ No DOI available (cannot download)")
                                };
                                let source_info = format!("\n  🔍 Source: {}", p.source);
                                let year = p.metadata.year.filter(|y| *y > 0)
                                    .map(|y| format!("\n  📅 Year: {}", y))
                                    .unwrap_or_default();
                                format!("{}. {} (Relevance: {:.0}%){}{}{}",
                                    i + 1,
                                    p.metadata.title.as_deref().unwrap_or("No title"),
                                    p.relevance_score * 100.0,
                                    doi_info,
                                    source_info,
                                    year
                                )
                            }).collect::<Vec<_>>().join("\n\n"),
                            results.papers.iter().filter(|p| !p.metadata.doi.is_empty()).count()
                        ))]),
                        structured_content: None,
                        is_error: Some(false),
                    })
                }
                "download_paper" => {
                    // Simple parsing for simplified schema
                    let args = request.arguments.unwrap_or_default();
                    let doi = args.get("doi").and_then(|v| v.as_str()).ok_or_else(|| {
                        ErrorData::invalid_params(
                            "Missing required 'doi' parameter".to_string(),
                            None,
                        )
                    })?;
                    let filename = args
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let input = ActualDownloadInput {
                        doi: Some(doi.to_string()),
                        url: None,
                        filename,
                        directory: None,
                        overwrite: false,
                        verify_integrity: true,
                    };

                    match download_tool.download_paper(input).await {
                        Ok(result) => {
                            // Validate that the file actually has content
                            let file_size = result.file_size.unwrap_or(0);
                            if file_size == 0 {
                                // Clean up zero-byte file if it exists
                                if let Some(file_path) = &result.file_path {
                                    if file_path.exists() {
                                        let _ = std::fs::remove_file(file_path);
                                    }
                                }
                                Ok(CallToolResult {
                                    content: Some(vec![Content::text(format!("⚠️ Download failed - no content received\n\nDOI: {}\n\nThe paper was found but no downloadable content is available. This could be because:\n• The paper is too new or recently published\n• It's behind a paywall not covered by available sources\n• The DOI might be incorrect\n\nTry checking the publisher's website or your institutional access.", doi))]),
                                    structured_content: None,
                                    is_error: Some(true),
                                })
                            } else {
                                Ok(CallToolResult {
                                    content: Some(vec![Content::text(format!("✅ Download successful!\n\n📄 File: {}\n📦 Size: {} KB\n✓ Status: Complete", 
                                        result.file_path.as_ref().map(|p| p.display().to_string()).unwrap_or("Unknown".to_string()),
                                        file_size / 1024))]),
                                    structured_content: None,
                                    is_error: Some(false),
                                })
                            }
                        }
                        Err(e) => {
                            // Return a helpful error message instead of failing the tool call
                            let error_msg = match e.to_string().as_str() {
                                msg if msg.contains("No PDF available") => {
                                    format!("⚠️ Paper not available on Sci-Hub\n\n\
                                            DOI: {}\n\n\
                                            This paper is not currently available through Sci-Hub. This could be because:\n\
                                            • The paper is too new (published recently)\n\
                                            • It's from a publisher not covered by Sci-Hub\n\
                                            • The DOI might be incorrect\n\n\
                                            Alternatives:\n\
                                            • Try searching for the paper on Google Scholar\n\
                                            • Check if your institution has access\n\
                                            • Try arXiv or other preprint servers\n\
                                            • Contact the authors directly", doi)
                                }
                                msg if msg.contains("Network") || msg.contains("timeout") => {
                                    format!("⚠️ Network error while downloading\n\n\
                                            Please check your internet connection and try again.\n\
                                            Error: {}", msg)
                                }
                                _ => {
                                    format!("⚠️ Download failed\n\n\
                                            DOI: {}\n\
                                            Error: {}\n\n\
                                            Please try again or use a different DOI.", doi, e)
                                }
                            };
                            Ok(CallToolResult {
                                content: Some(vec![Content::text(error_msg)]),
                                structured_content: None,
                                is_error: Some(true),
                            })
                        }
                    }
                }
                "extract_metadata" => {
                    let input: ActualMetadataInput = serde_json::from_value(
                        serde_json::Value::Object(request.arguments.unwrap_or_default()),
                    )
                    .map_err(|e| {
                        ErrorData::invalid_params(format!("Invalid metadata input: {}", e), None)
                    })?;

                    let result = metadata_extractor
                        .extract_metadata(input)
                        .await
                        .map_err(|e| {
                            ErrorData::internal_error(
                                format!("Metadata extraction failed: {}", e),
                                None,
                            )
                        })?;

                    Ok(CallToolResult {
                        content: Some(vec![Content::text(
                            serde_json::to_string_pretty(&result).map_err(|e| {
                                ErrorData::internal_error(
                                    format!("Serialization failed: {}", e),
                                    None,
                                )
                            })?,
                        )]),
                        structured_content: None,
                        is_error: Some(false),
                    })
                }
                _ => Err(ErrorData::invalid_request(
                    format!("Unknown tool: {}", request.name),
                    None,
                )),
            }
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

    fn create_test_handler() -> ResearchServerHandler {
        let config = Config::default();
        ResearchServerHandler::new(Arc::new(config)).unwrap()
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let handler = create_test_handler();
        assert!(handler.config.research_source.endpoints.len() > 0);
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
