use crate::client::{Doi, MetaSearchClient, PaperMetadata};
use crate::services::CategorizationService;
// use crate::tools::command::{Command, CommandResult, ExecutionContext};
use crate::{Config, Result};
// use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
// use tokio_util::io::ReaderStream; // Not needed currently
use tracing::{debug, error, info, instrument, warn};

/// Input parameters for the paper download tool
/// IMPORTANT: Either 'doi' or 'url' must be provided (not both optional!)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadInput {
    /// DOI of the paper to download (preferred - extract from `search_papers` results)
    #[schemars(
        description = "DOI of the paper (required if url not provided). Extract from search results."
    )]
    pub doi: Option<String>,
    /// Direct URL to download (alternative to DOI)
    #[schemars(description = "Direct download URL (required if doi not provided)")]
    pub url: Option<String>,
    /// Target filename (optional, will be derived if not provided)
    pub filename: Option<String>,
    /// Target directory (optional, uses default download directory)
    pub directory: Option<String>,
    /// Category for organizing the download (optional, creates subdirectory)
    pub category: Option<String>,
    /// Whether to overwrite existing files
    #[serde(default)]
    pub overwrite: bool,
    /// Whether to verify file integrity after download
    #[serde(default = "default_verify")]
    pub verify_integrity: bool,
}

/// Progress information for a download
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadProgress {
    /// Download ID for tracking
    pub download_id: String,
    /// DOI or URL being downloaded
    pub source: String,
    /// Total file size in bytes (if known)
    pub total_size: Option<u64>,
    /// Downloaded bytes so far
    pub downloaded: u64,
    /// Download percentage (0-100)
    pub percentage: f64,
    /// Current download speed in bytes/second
    pub speed_bps: u64,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u64>,
    /// Current status
    pub status: DownloadStatus,
    /// Target file path
    pub file_path: PathBuf,
    /// Error message if failed
    pub error: Option<String>,
}

/// Status of a download operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    /// Download is queued
    Queued,
    /// Download is in progress
    InProgress,
    /// Download completed successfully
    Completed,
    /// Download failed
    Failed,
    /// Download was paused
    Paused,
    /// Download was cancelled
    Cancelled,
}

/// Result of a download operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadResult {
    /// Download ID
    pub download_id: String,
    /// Final status
    pub status: DownloadStatus,
    /// Path to downloaded file
    pub file_path: Option<PathBuf>,
    /// File size in bytes
    pub file_size: Option<u64>,
    /// SHA256 hash of the file
    pub sha256_hash: Option<String>,
    /// Download duration in seconds
    pub duration_seconds: f64,
    /// Average download speed in bytes/second
    pub average_speed: u64,
    /// Paper metadata (if available)
    pub metadata: Option<PaperMetadata>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Download queue item
#[derive(Debug, Clone)]
pub struct DownloadQueueItem {
    pub id: String,
    pub input: DownloadInput,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
}

/// Internal download state
#[derive(Debug)]
#[allow(dead_code)] // Will be used for download tracking in future
struct DownloadState {
    progress: DownloadProgress,
    start_time: SystemTime,
    last_update: SystemTime,
    bytes_at_last_update: u64,
}

/// Progress callback type
pub type ProgressCallback = Arc<dyn Fn(DownloadProgress) + Send + Sync>;

/// Default for integrity verification
const fn default_verify() -> bool {
    true
}

/// Paper download tool implementation
#[derive(Clone)]
pub struct DownloadTool {
    client: Arc<MetaSearchClient>,
    http_client: Client,
    #[allow(dead_code)] // Will be used for configuration in future features
    config: Arc<Config>,
    download_queue: Arc<RwLock<Vec<DownloadQueueItem>>>,
    active_downloads: Arc<RwLock<HashMap<String, DownloadState>>>,
    progress_sender: Option<mpsc::UnboundedSender<DownloadProgress>>,
    categorization_service: CategorizationService,
}

impl std::fmt::Debug for DownloadTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadTool")
            .field("client", &"SciHubClient")
            .field("http_client", &"Client")
            .field("config", &"Config")
            .field("download_queue", &"RwLock<Vec<DownloadQueueItem>>")
            .field("active_downloads", &"RwLock<HashMap>")
            .field("progress_sender", &"Option<UnboundedSender>")
            .field("categorization_service", &"CategorizationService")
            .finish()
    }
}

impl DownloadTool {
    /// Create a new download tool
    pub fn new(client: Arc<MetaSearchClient>, config: Arc<Config>) -> Result<Self> {
        info!("Initializing paper download tool");

        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.research_source.timeout_secs * 2)) // Longer timeout for downloads
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10) // Enable connection pooling with 10 idle connections per host
            .pool_idle_timeout(Duration::from_secs(30)) // Keep idle connections for 30 seconds
            // Removed http2_prior_knowledge() to fix HTTP/2 frame size errors
            .http2_keep_alive_interval(Some(Duration::from_secs(30))) // Less aggressive HTTP/2 keepalive
            .tcp_keepalive(Some(Duration::from_secs(60))) // TCP keepalive
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36") // Add realistic user agent
            .build()
            .map_err(|e| crate::Error::Http(e))?;

        // Create categorization service
        let categorization_service = CategorizationService::new(config.categorization.clone())
            .map_err(|e| {
                crate::Error::Service(format!("Failed to create categorization service: {e}"))
            })?;

        Ok(Self {
            client,
            http_client,
            config,
            download_queue: Arc::new(RwLock::new(Vec::new())),
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            progress_sender: None,
            categorization_service,
        })
    }

    /// Set progress callback for download notifications
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.progress_sender = Some(sender);

        tokio::spawn(async move {
            while let Some(progress) = receiver.recv().await {
                callback(progress);
            }
        });
    }

    /// Download a paper by DOI or URL
    // #[tool] // Will be enabled when rmcp integration is complete
    #[instrument(skip(self), fields(doi = ?input.doi, url = ?input.url))]
    pub async fn download_paper(&self, input: DownloadInput) -> Result<DownloadResult> {
        debug!("📥 Starting paper download process");
        debug!("🔍 Input validation - DOI: {:?}, URL: {:?}, filename: {:?}, directory: {:?}, category: {:?}",
               input.doi, input.url, input.filename, input.directory, input.category);
        debug!(
            "⚙️ Download settings - overwrite: {}, verify_integrity: {}",
            input.overwrite, input.verify_integrity
        );

        info!(
            "Starting paper download: doi={:?}, url={:?}",
            input.doi, input.url
        );

        // Validate input
        debug!("🔍 Validating download input parameters");
        Self::validate_input(&input)?;
        debug!("✅ Input validation passed");

        // Generate download ID
        let download_id = uuid::Uuid::new_v4().to_string();
        debug!("🆔 Generated download ID: {}", download_id);

        // Get download URL and metadata
        debug!("🔎 Resolving download source for input");
        let (download_url, metadata) = match self.resolve_download_source(&input).await {
            Ok((url, meta)) => {
                debug!("✅ Successfully resolved download source");
                debug!("📄 Metadata found: {}", meta.is_some());
                debug!("🔗 Download URL length: {} chars", url.len());
                debug!(
                    "🔗 Download URL (truncated): {}...",
                    if url.len() > 100 { &url[..100] } else { &url }
                );
                (url, meta)
            }
            Err(e) => {
                debug!("❌ Failed to resolve download source: {}", e);
                debug!("🔧 Error type: {:?}", std::any::type_name_of_val(&e));
                return Err(e);
            }
        };

        // Safety check: ensure we never proceed with an empty URL
        if download_url.is_empty() {
            error!("❌ resolve_download_source returned an empty URL - this is a bug!");
            debug!(
                "🐛 Empty URL bug detected - this should never happen after successful resolution"
            );
            return Err(crate::Error::InvalidInput {
                field: "download_url".to_string(),
                reason: "Internal error: No download URL was found for this paper".to_string(),
            });
        }

        debug!("✅ URL safety check passed - proceeding with download");

        // Determine target file path
        debug!("📁 Determining target file path");
        let file_path = match self
            .determine_file_path(&input, metadata.as_ref(), &download_url)
            .await
        {
            Ok(path) => {
                debug!("✅ Target file path determined: {:?}", path);
                debug!(
                    "📁 Directory exists: {}",
                    path.parent().map_or(false, |p| p.exists())
                );
                path
            }
            Err(e) => {
                debug!("❌ Failed to determine file path: {}", e);
                debug!("🔧 Error details: {:?}", e);
                return Err(e);
            }
        };

        // Check for existing file
        debug!("🔍 Checking for existing file at: {:?}", file_path);
        if file_path.exists() && !input.overwrite {
            debug!("📄 File already exists, checking integrity verification setting");
            if input.verify_integrity {
                debug!("🔐 Calculating hash for existing file verification");
                if let Ok(hash) = self.calculate_file_hash(&file_path).await {
                    let file_size = tokio::fs::metadata(&file_path).await?.len();
                    debug!(
                        "✅ Existing file verified - size: {} bytes, hash: {}",
                        file_size,
                        &hash[..16]
                    );
                    info!("File already exists and verified: {:?}", file_path);
                    return Ok(DownloadResult {
                        download_id,
                        status: DownloadStatus::Completed,
                        file_path: Some(file_path),
                        file_size: Some(file_size),
                        sha256_hash: Some(hash),
                        duration_seconds: 0.0,
                        average_speed: 0,
                        metadata,
                        error: None,
                    });
                }
                debug!("⚠️ Failed to verify existing file hash");
            } else {
                debug!("❌ File exists and overwrite not enabled");
                return Err(crate::Error::InvalidInput {
                    field: "file_path".to_string(),
                    reason: format!(
                        "File already exists: {file_path}",
                        file_path = file_path.display()
                    ),
                });
            }
        } else if file_path.exists() {
            debug!("📄 File exists but overwrite is enabled - will replace");
        } else {
            debug!("✅ No existing file found - proceeding with fresh download");
        }

        // Perform the download
        debug!("🚀 Starting download execution");
        debug!(
            "📊 Download parameters - ID: {}, verify: {}, file: {:?}",
            download_id, input.verify_integrity, file_path
        );

        match self
            .execute_download(
                download_id.clone(),
                download_url,
                file_path,
                metadata,
                input.verify_integrity,
            )
            .await
        {
            Ok(result) => {
                debug!("✅ Download execution completed successfully");
                debug!(
                    "📊 Final result - status: {:?}, size: {:?} bytes, duration: {:.2}s",
                    result.status, result.file_size, result.duration_seconds
                );
                Ok(result)
            }
            Err(e) => {
                debug!("❌ Download execution failed: {}", e);
                debug!("🔧 Error type: {:?}", std::any::type_name_of_val(&e));
                debug!("📝 Full error context: {:?}", e);
                Err(e)
            }
        }
    }

    /// Validate download input
    fn validate_input(input: &DownloadInput) -> Result<()> {
        if input.doi.is_none() && input.url.is_none() {
            return Err(crate::Error::InvalidInput {
                field: "input".to_string(),
                reason: "Either DOI or URL must be provided".to_string(),
            });
        }

        if input.doi.is_some() && input.url.is_some() {
            return Err(crate::Error::InvalidInput {
                field: "input".to_string(),
                reason: "Cannot specify both DOI and URL".to_string(),
            });
        }

        // Validate DOI format if provided
        if let Some(doi_str) = &input.doi {
            Doi::new(doi_str)?;
        }

        // Validate URL format if provided
        if let Some(url_str) = &input.url {
            url::Url::parse(url_str).map_err(|e| crate::Error::InvalidInput {
                field: "url".to_string(),
                reason: format!("Invalid URL: {e}"),
            })?;
        }

        // Validate filename if provided - enhanced security checks
        if let Some(filename) = &input.filename {
            // Check for path traversal attempts
            if filename.contains("..")
                || filename.contains('/')
                || filename.contains('\\')
                || filename.contains(';')
                || filename.contains('|')
                || filename.contains('&')
                || filename.contains('`')
                || filename.contains('$')
                || filename.contains('>')
                || filename.contains('<')
                || filename.starts_with('-')
                || filename.contains("..\\")
                || filename.contains("....")
                || filename.contains("%2e%2e")
                || filename.contains("%2f")
                || filename.contains("%5c")
                || filename.is_empty()
                || filename.len() > 255
            {
                return Err(crate::Error::InvalidInput {
                    field: "filename".to_string(),
                    reason:
                        "Invalid filename: contains unsafe characters or path traversal attempts"
                            .to_string(),
                });
            }
            // Check for null bytes
            if filename.contains('\0') {
                return Err(crate::Error::InvalidInput {
                    field: "filename".to_string(),
                    reason: "Invalid filename: contains null bytes".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Resolve download source to URL and metadata
    async fn resolve_download_source(
        &self,
        input: &DownloadInput,
    ) -> Result<(String, Option<PaperMetadata>)> {
        if let Some(doi_str) = &input.doi {
            debug!("🆔 Starting DOI-based resolution for: {}", doi_str);
            info!("Attempting to download paper with DOI: {}", doi_str);

            // Create a search query for the DOI
            debug!("🔍 Creating search query for DOI resolution");
            let search_query = crate::client::providers::SearchQuery {
                query: doi_str.clone(),
                search_type: crate::client::providers::SearchType::Doi,
                max_results: 1,
                offset: 0,
                params: HashMap::new(),
            };
            debug!("🔍 Search query created - type: DOI, max_results: 1");

            // Use the meta search client to find papers across ALL providers
            debug!("🔎 Executing meta-search across all providers");
            let search_result = match self.client.search(&search_query).await {
                Ok(result) => {
                    debug!("✅ Meta-search completed successfully");
                    debug!("📊 Search stats - papers: {}, successful_providers: {}, failed_providers: {}",
                           result.papers.len(), result.successful_providers, result.failed_providers);
                    result
                }
                Err(e) => {
                    debug!("❌ Meta-search failed: {}", e);
                    debug!("🔧 Search error type: {:?}", std::any::type_name_of_val(&e));
                    return Err(e.into());
                }
            };

            info!(
                "Meta-search found {} papers from {} providers",
                search_result.papers.len(),
                search_result.successful_providers
            );

            // Log detailed provider results
            debug!("📋 Provider breakdown:");
            for (source, papers) in &search_result.by_source {
                debug!("• {}: {} papers", source, papers.len());
                if !papers.is_empty() {
                    for (i, paper) in papers.iter().enumerate().take(2) {
                        // Log first 2 papers max
                        debug!(
                            "  [{}] Title: {:?}, PDF URL present: {}",
                            i + 1,
                            paper
                                .title
                                .as_ref()
                                .map(|t| if t.len() > 50 { &t[..50] } else { t }),
                            paper.pdf_url.as_ref().map_or(false, |url| !url.is_empty())
                        );
                    }
                }
            }

            // First, look for any paper with a non-empty PDF URL already available
            debug!("🔍 Looking for papers with direct PDF URLs");
            let paper_with_pdf = search_result
                .papers
                .iter()
                .enumerate()
                .find_map(|(i, paper)| {
                    let has_pdf = paper
                        .pdf_url
                        .as_ref()
                        .map(|url| !url.is_empty())
                        .unwrap_or(false);
                    debug!("  Paper {}: PDF URL available: {}", i + 1, has_pdf);
                    if has_pdf {
                        debug!("  ✅ Found paper with direct PDF URL at index {}", i);
                        Some(paper.clone())
                    } else {
                        None
                    }
                });

            if let Some(paper) = paper_with_pdf {
                if let Some(pdf_url) = &paper.pdf_url {
                    if !pdf_url.is_empty() {
                        debug!("✅ Direct PDF URL found - length: {} chars", pdf_url.len());
                        debug!("🔗 URL source: direct provider response");
                        info!("Found PDF URL directly from provider: {}", pdf_url);
                        return Ok((pdf_url.clone(), Some(paper)));
                    }
                    debug!("⚠️ Paper has PDF URL field but it's empty - data inconsistency");
                    warn!("Paper has PDF URL but it's empty - this shouldn't happen!");
                } else {
                    debug!("⚠️ Paper found but pdf_url field is None");
                }
            } else {
                debug!("🔍 No papers with direct PDF URLs found in search results");
            }

            // If no direct PDF URL, try cascade approach through each provider
            debug!("🔄 Initiating cascade retrieval approach");
            info!("No direct PDF URL found, attempting cascade retrieval through all providers");

            // Log what we found from each source
            debug!("📋 Logging detailed source analysis:");
            for (source, papers) in &search_result.by_source {
                if !papers.is_empty() {
                    debug!(
                        "• Provider '{}' found {} paper(s) but no PDF URL",
                        source,
                        papers.len()
                    );
                    info!("Provider '{}' found paper metadata but no PDF URL", source);
                    for paper in papers {
                        debug!(
                            "    - Title: {:?}",
                            paper
                                .title
                                .as_ref()
                                .map(|t| if t.len() > 60 { &t[..60] } else { t })
                        );
                        debug!(
                            "    - Authors: {:?}",
                            paper.authors.iter().take(3).collect::<Vec<_>>()
                        );
                        debug!("    - Year: {:?}", paper.year);
                    }
                } else {
                    debug!("• Provider '{}' returned no results", source);
                }
            }

            // Try cascade PDF retrieval through all providers
            debug!("🔄 Executing cascade retrieval for DOI: {}", doi_str);
            match self.client.get_pdf_url_cascade(doi_str).await {
                Ok(Some(pdf_url)) => {
                    debug!("✅ Cascade retrieval SUCCESS! PDF URL obtained");
                    debug!("🔗 PDF URL length: {} chars", pdf_url.len());
                    debug!(
                        "📄 Using metadata from first search result: {}",
                        search_result.papers.first().is_some()
                    );
                    info!("Cascade retrieval successful! Found PDF URL: {}", pdf_url);
                    // Use the first paper's metadata if available
                    let metadata = search_result.papers.first().cloned();
                    return Ok((pdf_url, metadata));
                }
                Ok(None) => {
                    debug!("❌ Cascade retrieval completed but returned None");
                    debug!("📝 This means all providers were checked but no PDF was found");
                    info!("Cascade retrieval completed but no PDF found in any provider");
                }
                Err(e) => {
                    debug!("❌ Cascade retrieval failed with error: {}", e);
                    debug!(
                        "🔧 Cascade error type: {:?}",
                        std::any::type_name_of_val(&e)
                    );
                    warn!("Cascade retrieval failed with error: {}", e);
                }
            }

            // If cascade also failed, return detailed error with metadata
            debug!("❌ All retrieval methods exhausted - preparing detailed error response");
            if let Some(paper) = search_result.papers.first() {
                debug!("📄 Paper metadata found, checking for data inconsistencies");
                // Check if any paper has an empty PDF URL (shouldn't happen, but let's be safe)
                if let Some(empty_url_paper) = search_result.papers.iter().find(|p| {
                    p.pdf_url
                        .as_ref()
                        .map(|url| url.is_empty())
                        .unwrap_or(false)
                }) {
                    debug!("⚠️ Data inconsistency detected - paper with empty PDF URL field");
                    warn!(
                        "Found paper with empty PDF URL - this shouldn't happen! Paper: {:?}",
                        empty_url_paper
                    );
                }

                debug!("📋 Preparing detailed error message with paper metadata");
                debug!(
                    "📄 Paper details - Title: {:?}, Authors: {:?}, Year: {:?}",
                    paper.title, paper.authors, paper.year
                );

                let error_msg = format!(
                    "📄 Paper Metadata Found but No PDF Available\n\n\
                    The paper was successfully located in {} academic database(s), but none provided a downloadable PDF link.\n\n\
                    📚 Paper Details:\n\
                    • Title: '{}'\n\
                    • Authors: {}\n\
                    • Year: {}\n\
                    • DOI: {}\n\n\
                    🔍 Sources Searched: ArXiv, CrossRef, SSRN, Sci-Hub, and others\n\n\
                    💡 This typically means:\n\
                    • The paper is behind a paywall\n\
                    • It's a book or conference proceedings requiring institutional access\n\
                    • The paper may be available only in print\n\
                    • Publishers haven't made it freely available\n\n\
                    🚀 Try These Alternatives:\n\
                    1. Check your institution's library access\n\
                    2. Visit the publisher's website directly\n\
                    3. Search Google Scholar for preprint versions\n\
                    4. Contact the authors for a copy\n\
                    5. Check ResearchGate or Academia.edu\n\
                    6. Look for related open-access papers by the same authors",
                    search_result.successful_providers,
                    paper.title.as_ref().unwrap_or(&"Unknown title".to_string()),
                    if paper.authors.is_empty() { "Unknown authors".to_string() } else { paper.authors.join(", ") },
                    paper.year.map_or("Unknown year".to_string(), |y| y.to_string()),
                    doi_str
                );

                debug!(
                    "📝 Generated error message length: {} chars",
                    error_msg.len()
                );

                debug!("❌ Returning ServiceUnavailable error for PDF Download");
                Err(crate::Error::ServiceUnavailable {
                    service: "PDF Download".to_string(),
                    reason: error_msg,
                })
            } else {
                debug!("❌ No paper metadata found in any provider");
                debug!(
                    "📊 Search summary - successful: {}, failed: {}, total checked: {}",
                    search_result.successful_providers,
                    search_result.failed_providers,
                    search_result.successful_providers + search_result.failed_providers
                );

                let error_msg = format!(
                    "🔍 Paper Not Found in Academic Databases\n\n\
                    The DOI '{}' was not found in any of the {} academic databases we searched.\n\n\
                    📊 Search Summary:\n\
                    • Databases checked: {}\n\
                    • Databases that responded: {}\n\
                    • Databases that failed: {}\n\n\
                    💡 This could mean:\n\
                    • The DOI is incorrect or mistyped\n\
                    • The paper is very new and not yet indexed\n\
                    • The paper is in a specialized database we don't search\n\
                    • The DOI was registered but the paper was never published\n\n\
                    🔧 Try These Steps:\n\
                    1. Double-check the DOI format (should be like '10.1000/example')\n\
                    2. Search by paper title instead of DOI\n\
                    3. Check the original source where you found this DOI\n\
                    4. Try searching Google Scholar directly\n\
                    5. Contact the publisher or authors for verification",
                    doi_str,
                    search_result.successful_providers + search_result.failed_providers,
                    search_result.successful_providers + search_result.failed_providers,
                    search_result.successful_providers,
                    search_result.failed_providers
                );

                debug!(
                    "❌ Returning ServiceUnavailable error for MetaSearch: {}",
                    error_msg
                );
                Err(crate::Error::ServiceUnavailable {
                    service: "MetaSearch".to_string(),
                    reason: error_msg,
                })
            }
        } else if let Some(url) = &input.url {
            debug!("🔗 Using direct URL for download: {} chars", url.len());
            debug!(
                "🔗 URL (truncated): {}...",
                if url.len() > 100 { &url[..100] } else { url }
            );
            Ok((url.clone(), None))
        } else {
            debug!("❌ No download source specified in input");
            Err(crate::Error::InvalidInput {
                field: "input".to_string(),
                reason: "No download source specified".to_string(),
            })
        }
    }

    /// Determine the target file path for download
    async fn determine_file_path(
        &self,
        input: &DownloadInput,
        metadata: Option<&PaperMetadata>,
        download_url: &str,
    ) -> Result<PathBuf> {
        // Get base directory
        let mut base_dir = input
            .directory
            .as_ref()
            .map_or_else(|| self.get_default_download_directory(), PathBuf::from);

        // Add category subdirectory if provided
        if let Some(category) = &input.category {
            if self.categorization_service.is_enabled() {
                // Sanitize the category to ensure it's filesystem safe
                let sanitized_category = self.categorization_service.sanitize_category(category);

                // Resolve any conflicts with existing directories/files
                let final_category = self
                    .categorization_service
                    .resolve_category_conflict(&base_dir, &sanitized_category);

                base_dir = base_dir.join(final_category);
                info!("Using category subdirectory: {:?}", base_dir);
            }
        }

        // Security: Validate path security before creating directories
        Self::validate_directory_security(&base_dir).await?;

        // Ensure directory exists with better error handling
        if let Err(e) = tokio::fs::create_dir_all(&base_dir).await {
            // Check if this is a permissions issue (common with Claude Desktop sandbox)
            if e.to_string().contains("Read-only file system")
                || e.to_string().contains("Permission denied")
                || e.to_string().contains("Operation not permitted")
            {
                return Err(crate::Error::InvalidInput {
                    field: "permissions".to_string(),
                    reason: format!(
                        "❌ Claude Desktop Permission Required ❌\n\n\
                        Claude Desktop needs permission to access your Downloads folder.\n\n\
                        📋 To fix this:\n\
                        1. Open System Settings → Privacy & Security → Files and Folders\n\
                        2. Find 'Claude' in the list\n\
                        3. Enable 'Downloads Folder' permission\n\
                        4. Restart Claude Desktop\n\n\
                        💡 Alternative: Create a folder like ~/Documents/Research-Papers and update your config:\n\
                        • In config.toml: directory = \"~/Documents/Research-Papers\"\n\
                        • Or set environment variable: RSH_DOWNLOAD_DIRECTORY\n\n\
                        📁 Attempted directory: {base_dir:?}\n\
                        🔧 Error details: {e}"
                    ),
                });
            }
            // For other errors, still try fallback but with clearer messaging
            let fallback_dir = if let Some(home_dir) = dirs::home_dir() {
                home_dir.join("Documents").join("Research-Papers")
            } else {
                PathBuf::from("/tmp/papers")
            };

            warn!(
                "Primary directory failed, trying fallback: {:?}",
                fallback_dir
            );

            tokio::fs::create_dir_all(&fallback_dir)
                .await
                .map_err(|fallback_err| crate::Error::InvalidInput {
                    field: "download_directory".to_string(),
                    reason: format!(
                        "❌ Cannot create download directory ❌\n\n\
                            Neither the configured directory nor fallback location worked.\n\n\
                            💡 Try these solutions:\n\
                            1. Grant Claude Desktop folder permissions in System Settings\n\
                            2. Use a different directory: ~/Documents/Research-Papers\n\
                            3. Check disk space and permissions\n\n\
                            📁 Configured: {base_dir:?}\n\
                            📁 Fallback tried: {fallback_dir:?}\n\
                            🔧 Original error: {e}\n\
                            🔧 Fallback error: {fallback_err}"
                    ),
                })?;

            // Update the base_dir to the fallback
            base_dir = fallback_dir;
            info!("Using fallback directory: {:?}", base_dir);
        }

        // Determine filename
        let filename = input.filename.as_ref().map_or_else(
            || Self::generate_filename(metadata, download_url),
            Clone::clone,
        );

        Ok(base_dir.join(filename))
    }

    /// Get default download directory from config
    fn get_default_download_directory(&self) -> PathBuf {
        self.config.downloads.directory.clone()
    }

    /// Generate filename from metadata or URL
    fn generate_filename(metadata: Option<&PaperMetadata>, download_url: &str) -> String {
        if let Some(meta) = metadata {
            if let Some(title) = &meta.title {
                // Sanitize title for filename
                let sanitized = title
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == ' ' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("_");

                let truncated = if sanitized.len() > 50 {
                    sanitized[..50].to_string()
                } else {
                    sanitized
                };

                return format!("{truncated}.pdf");
            }
        }

        // Fallback: extract filename from URL or use timestamp
        if let Ok(url) = url::Url::parse(download_url) {
            if let Some(mut segments) = url.path_segments() {
                if let Some(last_segment) = segments.next_back() {
                    if Path::new(last_segment)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
                    {
                        return last_segment.to_string();
                    }
                }
            }
        }

        // Final fallback
        format!(
            "paper_{timestamp}.pdf",
            timestamp = chrono::Utc::now().timestamp()
        )
    }

    /// Execute the actual download
    #[allow(clippy::too_many_lines)] // Complex download logic needs to be in one place
    async fn execute_download(
        &self,
        download_id: String,
        download_url: String,
        file_path: PathBuf,
        metadata: Option<PaperMetadata>,
        verify_integrity: bool,
    ) -> Result<DownloadResult> {
        debug!("🚀 Execute download called with ID: {}", download_id);
        debug!("🔗 Download URL validation");

        // Validate that the URL is not empty
        if download_url.is_empty() {
            debug!("❌ Download URL is empty - this should not happen");
            return Err(crate::Error::InvalidInput {
                field: "download_url".to_string(),
                reason: "Download URL cannot be empty".to_string(),
            });
        }
        debug!(
            "✅ URL validation passed - length: {} chars",
            download_url.len()
        );

        let start_time = SystemTime::now();
        debug!("⏱️ Download timer started at: {:?}", start_time);

        info!("Starting download: {} -> {:?}", download_url, file_path);
        debug!("📁 Target file: {:?}", file_path);
        debug!("🔐 Integrity verification: {}", verify_integrity);
        debug!("📄 Metadata available: {}", metadata.is_some());

        // Create initial progress state
        debug!("📊 Creating initial progress state");
        let mut progress = Self::create_initial_progress(
            download_id.clone(),
            download_url.clone(),
            file_path.clone(),
        );
        debug!("📊 Progress state created - status: {:?}", progress.status);

        // Send initial progress
        debug!("📡 Sending initial progress notification");
        self.send_progress(progress.clone());

        // Make HEAD request to get file size
        debug!("🔍 Making HEAD request to determine file size");
        let total_size = match self.get_content_length(&download_url).await {
            Ok(size) => {
                debug!(
                    "✅ Content-Length determined: {} bytes ({:.2} MB)",
                    size,
                    size as f64 / 1_048_576.0
                );
                Some(size)
            }
            Err(e) => {
                debug!("⚠️ Could not determine content length: {}", e);
                debug!("📝 Will download without progress percentage");
                None
            }
        };
        progress.total_size = total_size;

        // Check for partial download (resume capability) but don't create file yet
        debug!("🔄 Checking for resume capability");
        let start_byte = if file_path.exists() {
            let existing_size = tokio::fs::metadata(&file_path).await?.len();
            debug!("📄 Existing file found - size: {} bytes", existing_size);
            debug!(
                "🔄 Will attempt to resume download from byte {}",
                existing_size
            );
            existing_size
        } else {
            debug!("🆕 No existing file - starting fresh download");
            0
        };
        progress.downloaded = start_byte;
        if start_byte > 0 {
            debug!("📊 Updated progress with existing bytes: {}", start_byte);
        }

        // Make download request first to verify it's valid
        debug!("🌐 Making download request with start_byte: {}", start_byte);
        let response = match self.make_download_request(&download_url, start_byte).await {
            Ok(resp) => {
                debug!("✅ Download request successful");
                debug!("📊 Response status: {}", resp.status());
                debug!("📋 Response headers count: {}", resp.headers().len());
                if let Some(content_type) = resp.headers().get("content-type") {
                    debug!("📄 Content-Type: {:?}", content_type);
                }
                resp
            }
            Err(e) => {
                debug!("❌ Download request failed: {}", e);
                debug!(
                    "🔧 Request error type: {:?}",
                    std::any::type_name_of_val(&e)
                );
                return Err(e);
            }
        };

        // Update total size from response if not known
        debug!("🔄 Updating total size from response headers");
        let old_total = progress.total_size;
        Self::update_total_size_from_response(&mut progress, &response, start_byte);
        if progress.total_size != old_total {
            debug!(
                "📊 Total size updated from {} to {:?}",
                old_total.map_or("None".to_string(), |s| s.to_string()),
                progress.total_size
            );
        } else {
            debug!("📊 Total size unchanged: {:?}", progress.total_size);
        }

        // Download with progress tracking - this will create the file only if download succeeds
        debug!("📥 Starting progress-tracked download");
        match self
            .download_with_progress(response, &file_path, start_byte, &mut progress)
            .await
        {
            Ok(()) => {
                debug!("✅ Progress-tracked download completed successfully");
            }
            Err(e) => {
                debug!("❌ Progress-tracked download failed: {}", e);
                debug!(
                    "🔧 Download error type: {:?}",
                    std::any::type_name_of_val(&e)
                );
                return Err(e);
            }
        };

        // Finalize download
        debug!("🏁 Finalizing download process");
        match self
            .finalize_download(
                &file_path,
                start_time,
                verify_integrity,
                progress,
                download_id,
                metadata,
            )
            .await
        {
            Ok(result) => {
                debug!("✅ Download finalization completed successfully");
                debug!("📊 Final download stats - size: {:?} bytes, duration: {:.2}s, speed: {} bytes/s",
                       result.file_size, result.duration_seconds, result.average_speed);
                Ok(result)
            }
            Err(e) => {
                debug!("❌ Download finalization failed: {}", e);
                debug!(
                    "🔧 Finalization error type: {:?}",
                    std::any::type_name_of_val(&e)
                );
                Err(e)
            }
        }
    }

    /// Create initial progress state
    const fn create_initial_progress(
        download_id: String,
        download_url: String,
        file_path: PathBuf,
    ) -> DownloadProgress {
        DownloadProgress {
            download_id,
            source: download_url,
            total_size: None,
            downloaded: 0,
            percentage: 0.0,
            speed_bps: 0,
            eta_seconds: None,
            status: DownloadStatus::InProgress,
            file_path,
            error: None,
        }
    }

    /// Make download request with optional range header
    async fn make_download_request(
        &self,
        download_url: &str,
        start_byte: u64,
    ) -> Result<reqwest::Response> {
        let response = if start_byte > 0 {
            self.http_client
                .get(download_url)
                .header("Range", format!("bytes={start_byte}-"))
                .send()
                .await
        } else {
            self.http_client.get(download_url).send().await
        }
        .map_err(|e| crate::Error::Service(format!("Download request failed: {e}")))?;

        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(crate::Error::SciHub {
                code: response.status().as_u16(),
                message: "Download failed".to_string(),
            });
        }

        Ok(response)
    }

    /// Update total size from response headers
    fn update_total_size_from_response(
        progress: &mut DownloadProgress,
        response: &reqwest::Response,
        start_byte: u64,
    ) {
        if progress.total_size.is_none() {
            if let Some(content_length) = response.headers().get("content-length") {
                if let Ok(length_str) = content_length.to_str() {
                    if let Ok(length) = length_str.parse::<u64>() {
                        progress.total_size = Some(length + start_byte);
                    }
                }
            }
        }
    }

    /// Download with progress tracking
    async fn download_with_progress(
        &self,
        response: reqwest::Response,
        file_path: &PathBuf,
        start_byte: u64,
        progress: &mut DownloadProgress,
    ) -> Result<()> {
        debug!("📥 Starting progressive download");
        debug!("📁 Target file: {:?}", file_path);
        debug!("🔄 Resume from byte: {}", start_byte);
        debug!("📊 Expected total size: {:?}", progress.total_size);

        let mut stream = response.bytes_stream();
        let mut last_progress_time = SystemTime::now();
        let mut bytes_at_last_time = progress.downloaded;
        let mut chunk_count = 0u64;
        let mut total_bytes_received = 0u64;

        // Only create/open file when we start receiving data
        let mut file_created = false;
        let mut file: Option<File> = None;
        debug!("🔍 File will be created on first successful chunk");

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => {
                    chunk_count += 1;
                    total_bytes_received += chunk.len() as u64;
                    if chunk_count <= 5 || chunk_count % 100 == 0 {
                        debug!(
                            "📦 Chunk #{}: {} bytes (total: {} bytes)",
                            chunk_count,
                            chunk.len(),
                            total_bytes_received
                        );
                    }
                    chunk
                }
                Err(e) => {
                    debug!("❌ Stream error at chunk #{}: {}", chunk_count, e);
                    debug!("📊 Bytes received before error: {}", total_bytes_received);
                    return Err(crate::Error::Service(format!("Download stream error: {e}")));
                }
            };

            // Create file on first successful chunk
            if file_created {
                // File already created, write subsequent chunks
                if let Some(ref mut f) = file {
                    match f.write_all(&chunk).await {
                        Ok(()) => {
                            if chunk_count <= 3 {
                                debug!("✅ Chunk #{} written successfully", chunk_count);
                            }
                        }
                        Err(e) => {
                            debug!("❌ Failed to write chunk #{}: {}", chunk_count, e);
                            return Err(crate::Error::Io(e));
                        }
                    }
                }
            } else {
                debug!("📁 Creating/opening file for first chunk");
                let mut file_handle = if file_path.exists() && start_byte > 0 {
                    debug!("🔄 Resuming download - opening existing file for append");
                    // File exists, open for append
                    OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(file_path)
                        .await
                        .map_err(crate::Error::Io)?
                } else {
                    debug!("🆕 Creating new file for download");
                    // Security: Validate file path security before creation
                    Self::validate_file_security(file_path).await?;
                    debug!("✅ File security validation passed");

                    // Create new file only when we have data to write
                    let file = File::create(file_path).await.map_err(crate::Error::Io)?;
                    debug!("✅ File created successfully: {:?}", file_path);

                    // Security: Set restrictive permissions on downloaded files
                    Self::set_secure_file_permissions(file_path).await?;
                    debug!("✅ Secure file permissions set");

                    file
                };

                // Write the first chunk
                debug!("✏️ Writing first chunk ({} bytes)", chunk.len());
                match file_handle.write_all(&chunk).await {
                    Ok(()) => {
                        debug!("✅ First chunk written successfully");
                        file = Some(file_handle);
                        file_created = true;
                    }
                    Err(e) => {
                        debug!("❌ Failed to write first chunk: {}", e);
                        return Err(crate::Error::Io(e));
                    }
                }
            }

            progress.downloaded += chunk.len() as u64;

            // Update progress every 500ms
            let now = SystemTime::now();
            if now
                .duration_since(last_progress_time)
                .unwrap_or(Duration::ZERO)
                >= Duration::from_millis(500)
            {
                Self::update_progress_stats(progress, now, last_progress_time, bytes_at_last_time);
                debug!("📊 Progress update - downloaded: {} bytes, speed: {} bytes/s, percentage: {:.1}%",
                       progress.downloaded, progress.speed_bps, progress.percentage);
                self.send_progress(progress.clone());

                last_progress_time = now;
                bytes_at_last_time = progress.downloaded;
            }
        }

        // Ensure we actually received some data
        if !file_created {
            debug!("❌ No file was created - no data received from stream");
            debug!(
                "📊 Final stats - chunks: {}, total bytes: {}",
                chunk_count, total_bytes_received
            );
            return Err(crate::Error::Service(
                "No data received from download".to_string(),
            ));
        }

        debug!("✅ Download stream completed successfully");
        debug!(
            "📊 Final stats - {} chunks processed, {} bytes total",
            chunk_count, total_bytes_received
        );
        debug!("📁 File created at: {:?}", file_path);

        // Final progress update
        debug!("📡 Sending final progress update");
        Self::update_progress_stats(
            progress,
            SystemTime::now(),
            last_progress_time,
            bytes_at_last_time,
        );
        self.send_progress(progress.clone());

        Ok(())
    }

    /// Update progress statistics
    fn update_progress_stats(
        progress: &mut DownloadProgress,
        now: SystemTime,
        last_time: SystemTime,
        bytes_at_last_time: u64,
    ) {
        let elapsed = now
            .duration_since(last_time)
            .unwrap_or(Duration::from_secs(1));
        let bytes_diff = progress.downloaded - bytes_at_last_time;
        #[allow(clippy::cast_precision_loss)]
        let speed = bytes_diff as f64 / elapsed.as_secs_f64();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            progress.speed_bps = speed as u64;
        }

        if let Some(total) = progress.total_size {
            #[allow(clippy::cast_precision_loss)]
            let percentage = (progress.downloaded as f64 / total as f64) * 100.0;
            progress.percentage = percentage;
            let remaining_bytes = total - progress.downloaded;
            if progress.speed_bps > 0 {
                progress.eta_seconds = Some(remaining_bytes / progress.speed_bps);
            }
        }
    }

    /// Finalize download and create result
    async fn finalize_download(
        &self,
        file_path: &Path,
        start_time: SystemTime,
        verify_integrity: bool,
        mut progress: DownloadProgress,
        download_id: String,
        metadata: Option<PaperMetadata>,
    ) -> Result<DownloadResult> {
        // File was already properly closed by the download_with_progress method

        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        let file_size = tokio::fs::metadata(file_path).await?.len();
        let average_speed = if duration.as_secs() > 0 {
            file_size / duration.as_secs()
        } else {
            0
        };

        // Verify integrity if requested
        let sha256_hash = if verify_integrity {
            Some(self.calculate_file_hash(file_path).await?)
        } else {
            None
        };

        progress.status = DownloadStatus::Completed;
        progress.percentage = 100.0;
        self.send_progress(progress);

        info!("Download completed: {:?} ({} bytes)", file_path, file_size);

        Ok(DownloadResult {
            download_id,
            status: DownloadStatus::Completed,
            file_path: Some(file_path.to_path_buf()),
            file_size: Some(file_size),
            sha256_hash,
            duration_seconds: duration.as_secs_f64(),
            average_speed,
            metadata,
            error: None,
        })
    }

    /// Get content length from URL
    async fn get_content_length(&self, url: &str) -> Result<u64> {
        let response = self
            .http_client
            .head(url)
            .send()
            .await
            .map_err(|e| crate::Error::Service(format!("HEAD request failed: {e}")))?;

        if let Some(content_length) = response.headers().get("content-length") {
            let length_str = content_length.to_str().map_err(|e| crate::Error::Parse {
                context: "content-length header".to_string(),
                message: format!("Invalid content-length header: {e}"),
            })?;
            length_str.parse::<u64>().map_err(|e| crate::Error::Parse {
                context: "content-length value".to_string(),
                message: format!("Cannot parse content-length: {e}"),
            })
        } else {
            Err(crate::Error::Parse {
                context: "HTTP headers".to_string(),
                message: "No content-length header found".to_string(),
            })
        }
    }

    /// Calculate SHA256 hash of a file
    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await.map_err(crate::Error::Io)?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await.map_err(crate::Error::Io)?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Send progress update
    fn send_progress(&self, progress: DownloadProgress) {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(progress);
        }
    }

    /// Get active downloads
    pub async fn get_active_downloads(&self) -> Vec<DownloadProgress> {
        let downloads = self.active_downloads.read().await;
        downloads
            .values()
            .map(|state| state.progress.clone())
            .collect()
    }

    /// Cancel a download
    pub async fn cancel_download(&self, download_id: &str) -> Result<()> {
        let mut downloads = self.active_downloads.write().await;
        if let Some(mut state) = downloads.remove(download_id) {
            state.progress.status = DownloadStatus::Cancelled;
            self.send_progress(state.progress);
            info!("Download cancelled: {}", download_id);
            Ok(())
        } else {
            Err(crate::Error::InvalidInput {
                field: "download_id".to_string(),
                reason: format!("Download not found: {download_id}"),
            })
        }
    }

    /// Get download queue status
    pub async fn get_queue_status(&self) -> Vec<DownloadQueueItem> {
        let queue = self.download_queue.read().await;
        queue.clone()
    }

    /// Clear completed downloads from tracking
    pub async fn clear_completed(&self) {
        let mut downloads = self.active_downloads.write().await;
        downloads.retain(|_, state| {
            !matches!(
                state.progress.status,
                DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
            )
        });
        debug!(
            "Cleared completed downloads, {} active remaining",
            downloads.len()
        );
    }

    /// Validate directory security to prevent attacks
    async fn validate_directory_security(path: &Path) -> Result<()> {
        // Define trusted system symlinks that are safe on macOS
        #[cfg(target_os = "macos")]
        const TRUSTED_SYMLINKS: &[&str] = &["/var", "/tmp", "/etc", "/private"];

        // Check if any component in the path is a symbolic link
        let mut current_path = PathBuf::new();
        for component in path.components() {
            current_path.push(component);
            if current_path.exists() {
                let metadata = tokio::fs::symlink_metadata(&current_path)
                    .await
                    .map_err(|e| {
                        crate::Error::Service(format!("Failed to check path metadata: {e}"))
                    })?;

                if metadata.file_type().is_symlink() {
                    let path_str = current_path.to_string_lossy();

                    // On macOS, allow trusted system symlinks
                    #[cfg(target_os = "macos")]
                    {
                        let is_trusted = TRUSTED_SYMLINKS.iter().any(|&trusted| {
                            path_str == trusted || path_str.starts_with(&format!("{}/", trusted))
                        });

                        if !is_trusted {
                            return Err(crate::Error::Service(format!(
                                "Security: Refusing to create directory - path contains untrusted symbolic link: {:?}",
                                current_path
                            )));
                        }
                    }

                    // On other platforms, reject all symlinks
                    #[cfg(not(target_os = "macos"))]
                    {
                        return Err(crate::Error::Service(format!(
                            "Security: Refusing to create directory - path contains symbolic link: {:?}",
                            current_path
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate file path security before creation
    async fn validate_file_security(file_path: &Path) -> Result<()> {
        // Check if file already exists and is a symlink
        if file_path.exists() {
            let metadata = tokio::fs::symlink_metadata(file_path).await.map_err(|e| {
                crate::Error::Service(format!("Failed to check file metadata: {e}"))
            })?;

            if metadata.file_type().is_symlink() {
                return Err(crate::Error::Service(format!(
                    "Security: Refusing to overwrite symbolic link: {:?}",
                    file_path
                )));
            }
        }

        // Check parent directory for symlinks
        if let Some(parent) = file_path.parent() {
            Self::validate_directory_security(parent).await?;
        }

        Ok(())
    }

    /// Set secure file permissions on downloaded files
    async fn set_secure_file_permissions(file_path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // Set permissions to 0600 (owner read/write only) for downloaded files
            let permissions = std::fs::Permissions::from_mode(0o600);
            tokio::fs::set_permissions(file_path, permissions)
                .await
                .map_err(|e| {
                    crate::Error::Service(format!(
                        "Failed to set secure permissions on downloaded file: {e}"
                    ))
                })?;

            info!(
                "Set secure permissions (0600) on downloaded file: {:?}",
                file_path
            );
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, permissions are handled differently
            info!(
                "Non-Unix system: Cannot set Unix-style permissions on file: {:?}",
                file_path
            );
        }

        Ok(())
    }
}

// Command trait implementation for DownloadTool (temporarily disabled)
/*
#[async_trait]
impl Command for DownloadTool {
    fn name(&self) -> &'static str {
        "download_paper"
    }

    fn description(&self) -> &'static str {
        "Download academic papers by DOI or direct URL with integrity verification and progress tracking"
    }

    fn input_schema(&self) -> serde_json::Value {
        use schemars::schema_for;
        let schema = schema_for!(DownloadInput);
        serde_json::to_value(schema).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize DownloadInput schema: {}", e);
            serde_json::json!({
                "type": "object",
                "properties": {
                    "doi": {"type": "string", "description": "DOI of the paper (required if url not provided)"},
                    "url": {"type": "string", "description": "Direct download URL (required if doi not provided)"},
                    "filename": {"type": "string", "description": "Target filename (optional)"},
                    "directory": {"type": "string", "description": "Target directory (optional)"},
                    "category": {"type": "string", "description": "Category for organizing downloads (optional)"},
                    "overwrite": {"type": "boolean", "default": false},
                    "verify_integrity": {"type": "boolean", "default": true}
                },
                "anyOf": [
                    {"required": ["doi"]},
                    {"required": ["url"]}
                ]
            })
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        use schemars::schema_for;
        let schema = schema_for!(DownloadResult);
        serde_json::to_value(schema).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize DownloadResult schema: {}", e);
            serde_json::json!({
                "type": "object",
                "properties": {
                    "download_id": {"type": "string"},
                    "status": {"type": "string"},
                    "file_path": {"type": "string"},
                    "file_size": {"type": "integer"},
                    "sha256_hash": {"type": "string"},
                    "duration_seconds": {"type": "number"},
                    "average_speed": {"type": "integer"},
                    "metadata": {"type": "object"}
                }
            })
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> Result<CommandResult> {
        let start_time = SystemTime::now();

        // Deserialize input
        let download_input: DownloadInput =
            serde_json::from_value(input).map_err(|e| crate::Error::InvalidInput {
                field: "input".to_string(),
                reason: format!("Invalid download input: {e}"),
            })?;

        // Check for timeout before starting download
        if context.is_timed_out() {
            let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
            return Ok(CommandResult::failure(
                context.request_id,
                self.name().to_string(),
                "Command timed out before download could start".to_string(),
                duration,
            ));
        }

        // Execute the download
        let download_result = self.download_paper(download_input).await?;

        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);

        // Create successful command result with additional metadata
        let mut result = CommandResult::success(
            context.request_id,
            self.name().to_string(),
            download_result,
            duration,
        )?;

        // Add download-specific metadata
        result = result
            .with_metadata("operation_type".to_string(), "file_download".to_string())
            .with_metadata("has_progress_tracking".to_string(), "true".to_string());

        Ok(result)
    }

    async fn validate_input(&self, input: &serde_json::Value) -> Result<()> {
        // Try to deserialize to check basic structure
        let download_input: DownloadInput =
            serde_json::from_value(input.clone()).map_err(|e| crate::Error::InvalidInput {
                field: "input".to_string(),
                reason: format!("Invalid input structure: {e}"),
            })?;

        // Use existing validation logic
        Self::validate_input(&download_input)
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(60) // Downloads can take 1-5 minutes depending on file size
    }

    fn is_concurrent_safe(&self) -> bool {
        true // Downloads are safe to run concurrently
    }

    fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "validation"
            | "timeout"
            | "metadata"
            | "progress_tracking"
            | "integrity_verification" => true,
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ResearchSourceConfig};
    // use std::path::PathBuf; // Already imported at top level
    use tempfile::TempDir;

    fn create_test_config() -> Arc<Config> {
        let mut config = Config::default();
        config.research_source = ResearchSourceConfig {
            endpoints: vec!["https://sci-hub.se".to_string()],
            rate_limit_per_sec: 1,
            timeout_secs: 30,
            provider_timeout_secs: 60,
            max_retries: 2,
        };
        Arc::new(config)
    }

    fn create_test_download_tool() -> Result<DownloadTool> {
        let config = create_test_config();
        let meta_config = crate::client::MetaSearchConfig::from_config(&config);
        let client = Arc::new(MetaSearchClient::new((*config).clone(), meta_config)?);
        DownloadTool::new(client, config)
    }

    #[test]
    fn test_download_input_validation() {
        // No DOI or URL should fail
        let empty_input = DownloadInput {
            doi: None,
            url: None,
            filename: None,
            directory: None,
            category: None,
            overwrite: false,
            verify_integrity: true,
        };
        assert!(DownloadTool::validate_input(&empty_input).is_err());

        // Both DOI and URL should fail
        let both_input = DownloadInput {
            doi: Some("10.1038/nature12373".to_string()),
            url: Some("https://example.com/paper.pdf".to_string()),
            filename: None,
            directory: None,
            category: None,
            overwrite: false,
            verify_integrity: true,
        };
        assert!(DownloadTool::validate_input(&both_input).is_err());

        // Valid DOI should pass
        let valid_doi = DownloadInput {
            doi: Some("10.1038/nature12373".to_string()),
            url: None,
            filename: None,
            directory: None,
            category: None,
            overwrite: false,
            verify_integrity: true,
        };
        assert!(DownloadTool::validate_input(&valid_doi).is_ok());

        // Valid URL should pass
        let valid_url = DownloadInput {
            doi: None,
            url: Some("https://example.com/paper.pdf".to_string()),
            filename: None,
            directory: None,
            category: None,
            overwrite: false,
            verify_integrity: true,
        };
        assert!(DownloadTool::validate_input(&valid_url).is_ok());

        // Invalid filename should fail
        let invalid_filename = DownloadInput {
            doi: Some("10.1038/nature12373".to_string()),
            url: None,
            filename: Some("../malicious.pdf".to_string()),
            directory: None,
            category: None,
            overwrite: false,
            verify_integrity: true,
        };
        assert!(DownloadTool::validate_input(&invalid_filename).is_err());
    }

    #[test]
    fn test_filename_generation() {
        // Test with metadata
        let mut metadata = PaperMetadata::new("10.1038/test".to_string());
        metadata.title = Some(
            "A Very Long Paper Title That Should Be Truncated Because It Exceeds Fifty Characters"
                .to_string(),
        );

        let filename =
            DownloadTool::generate_filename(Some(&metadata), "https://example.com/test.pdf");
        assert!(filename.ends_with(".pdf"));
        assert!(filename.len() <= 54); // 50 chars + ".pdf"

        // Test with URL fallback
        let filename_url = DownloadTool::generate_filename(None, "https://example.com/paper.pdf");
        assert_eq!(filename_url, "paper.pdf");

        // Test with timestamp fallback
        let filename_fallback = DownloadTool::generate_filename(None, "https://example.com/");
        assert!(filename_fallback.starts_with("paper_"));
        assert!(filename_fallback.ends_with(".pdf"));
    }

    #[tokio::test]
    async fn test_default_download_directory() {
        let tool = create_test_download_tool().unwrap();
        let dir = tool.get_default_download_directory();
        // Check that it uses the config directory (which defaults to ~/Downloads/papers)
        assert!(dir.to_string_lossy().contains("papers"));
    }

    #[tokio::test]
    async fn test_file_path_determination() {
        let tool = create_test_download_tool().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let input = DownloadInput {
            doi: Some("10.1038/test".to_string()),
            url: None,
            filename: Some("test.pdf".to_string()),
            directory: Some(temp_dir.path().to_string_lossy().to_string()),
            category: None,
            overwrite: false,
            verify_integrity: true,
        };

        let metadata = Some(PaperMetadata::new("10.1038/test".to_string()));
        let file_path = tool
            .determine_file_path(&input, metadata.as_ref(), "https://example.com/test.pdf")
            .await
            .unwrap();

        assert_eq!(file_path.file_name().unwrap(), "test.pdf");
        assert!(file_path.starts_with(temp_dir.path()));
    }

    #[tokio::test]
    async fn test_file_hash_calculation() {
        let tool = create_test_download_tool().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        tokio::fs::write(&file_path, b"hello world").await.unwrap();
        let hash = tool.calculate_file_hash(&file_path).await.unwrap();

        // Known SHA256 hash of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[tokio::test]
    async fn test_download_tracking() {
        let tool = create_test_download_tool().unwrap();

        // Initially no active downloads
        let active = tool.get_active_downloads().await;
        assert!(active.is_empty());

        // Queue should be empty
        let queue = tool.get_queue_status().await;
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_custom_download_directory() {
        // Create config with custom download directory
        let mut config = Config::default();
        config.downloads.directory = PathBuf::from("/tmp/test-downloads");
        let meta_config = crate::client::MetaSearchConfig::from_config(&config);
        let client = Arc::new(MetaSearchClient::new(config.clone(), meta_config).unwrap());
        let tool = DownloadTool::new(client, Arc::new(config)).unwrap();

        // Test that the tool uses the custom directory
        let dir = tool.get_default_download_directory();
        assert_eq!(dir, PathBuf::from("/tmp/test-downloads"));

        // Test file path determination with no override
        let input = DownloadInput {
            doi: Some("10.1038/test".to_string()),
            url: None,
            filename: Some("test.pdf".to_string()),
            directory: None, // No override, should use config default
            category: None,
            overwrite: false,
            verify_integrity: false,
        };

        let metadata = PaperMetadata::new("10.1038/test".to_string());
        let file_path = tool
            .determine_file_path(&input, Some(&metadata), "https://example.com/test.pdf")
            .await
            .unwrap();

        // Should use the custom directory from config
        assert!(file_path.starts_with("/tmp/test-downloads"));
        assert!(file_path.ends_with("test.pdf"));
    }
}
