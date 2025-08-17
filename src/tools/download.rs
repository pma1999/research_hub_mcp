use crate::client::{Doi, MetaSearchClient, PaperMetadata};
use crate::{Config, Result};
use futures::StreamExt;
use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};
// use tokio_util::io::ReaderStream; // Not needed currently
use tracing::{debug, info, instrument};

/// Input parameters for the paper download tool
/// IMPORTANT: Either 'doi' or 'url' must be provided (not both optional!)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DownloadInput {
    /// DOI of the paper to download (preferred - extract from search_papers results)
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
            .finish()
    }
}

impl DownloadTool {
    /// Create a new download tool
    pub fn new(client: Arc<MetaSearchClient>, config: Arc<Config>) -> Self {
        info!("Initializing paper download tool");

        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.research_source.timeout_secs * 2)) // Longer timeout for downloads
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for downloads");

        Self {
            client,
            http_client,
            config,
            download_queue: Arc::new(RwLock::new(Vec::new())),
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            progress_sender: None,
        }
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
        info!(
            "Starting paper download: doi={:?}, url={:?}",
            input.doi, input.url
        );

        // Validate input
        Self::validate_input(&input)?;

        // Generate download ID
        let download_id = uuid::Uuid::new_v4().to_string();

        // Get download URL and metadata
        let (download_url, metadata) = self.resolve_download_source(&input).await?;

        // Determine target file path
        let file_path = self
            .determine_file_path(&input, metadata.as_ref(), &download_url)
            .await?;

        // Check for existing file
        if file_path.exists() && !input.overwrite {
            if input.verify_integrity {
                if let Ok(hash) = self.calculate_file_hash(&file_path).await {
                    info!("File already exists and verified: {:?}", file_path);
                    let file_size = tokio::fs::metadata(&file_path).await?.len();
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
            } else {
                return Err(crate::Error::InvalidInput {
                    field: "file_path".to_string(),
                    reason: format!("File already exists: {}", file_path.display()),
                });
            }
        }

        // Perform the download
        self.execute_download(
            download_id.clone(),
            download_url,
            file_path,
            metadata,
            input.verify_integrity,
        )
        .await
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

        // Validate filename if provided
        if let Some(filename) = &input.filename {
            if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                return Err(crate::Error::InvalidInput {
                    field: "filename".to_string(),
                    reason: "Invalid filename: cannot contain path separators".to_string(),
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
            // Create a search query for the DOI
            let search_query = crate::client::providers::SearchQuery {
                query: doi_str.clone(),
                search_type: crate::client::providers::SearchType::Doi,
                max_results: 1,
                offset: 0,
                params: HashMap::new(),
            };

            // Use the meta search client to find papers with PDF URLs
            let search_result = self.client.search(&search_query).await?;

            // Look for a paper with a PDF URL (prioritizing Sci-Hub results)
            let paper_with_pdf = search_result
                .papers
                .iter()
                .find(|paper| paper.pdf_url.is_some())
                .cloned();

            if let Some(paper) = paper_with_pdf {
                if let Some(pdf_url) = &paper.pdf_url {
                    Ok((pdf_url.clone(), Some(paper)))
                } else {
                    Err(crate::Error::ServiceUnavailable {
                        service: "MetaSearch".to_string(),
                        reason: format!("No PDF available for DOI: {doi_str}"),
                    })
                }
            } else {
                Err(crate::Error::ServiceUnavailable {
                    service: "MetaSearch".to_string(),
                    reason: format!("No PDF available for DOI: {doi_str}"),
                })
            }
        } else if let Some(url) = &input.url {
            Ok((url.clone(), None))
        } else {
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
        let base_dir = input
            .directory
            .as_ref()
            .map_or_else(|| self.get_default_download_directory(), PathBuf::from);

        // Ensure directory exists
        tokio::fs::create_dir_all(&base_dir)
            .await
            .map_err(crate::Error::Io)?;

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
        format!("paper_{}.pdf", chrono::Utc::now().timestamp())
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
        let start_time = SystemTime::now();

        info!("Starting download: {} -> {:?}", download_url, file_path);

        // Create initial progress state
        let mut progress = Self::create_initial_progress(
            download_id.clone(),
            download_url.clone(),
            file_path.clone(),
        );

        // Send initial progress
        self.send_progress(progress.clone());

        // Make HEAD request to get file size
        let total_size = self.get_content_length(&download_url).await.ok();
        progress.total_size = total_size;

        // Check for partial download (resume capability)
        let (mut file, start_byte) = self.prepare_download_file(&file_path).await?;
        progress.downloaded = start_byte;

        // Make download request
        let response = self
            .make_download_request(&download_url, start_byte)
            .await?;

        // Update total size from response if not known
        Self::update_total_size_from_response(&mut progress, &response, start_byte);

        // Download with progress tracking
        self.download_with_progress(response, &mut file, &mut progress)
            .await?;

        // Finalize download
        self.finalize_download(
            file,
            &file_path,
            start_time,
            verify_integrity,
            progress,
            download_id,
            metadata,
        )
        .await
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
        file: &mut File,
        progress: &mut DownloadProgress,
    ) -> Result<()> {
        let mut stream = response.bytes_stream();
        let mut last_progress_time = SystemTime::now();
        let mut bytes_at_last_time = progress.downloaded;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| crate::Error::Service(format!("Download stream error: {e}")))?;

            file.write_all(&chunk).await.map_err(crate::Error::Io)?;

            progress.downloaded += chunk.len() as u64;

            // Update progress every 500ms
            let now = SystemTime::now();
            if now
                .duration_since(last_progress_time)
                .unwrap_or(Duration::ZERO)
                >= Duration::from_millis(500)
            {
                Self::update_progress_stats(progress, now, last_progress_time, bytes_at_last_time);
                self.send_progress(progress.clone());

                last_progress_time = now;
                bytes_at_last_time = progress.downloaded;
            }
        }

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
        mut file: File,
        file_path: &Path,
        start_time: SystemTime,
        verify_integrity: bool,
        mut progress: DownloadProgress,
        download_id: String,
        metadata: Option<PaperMetadata>,
    ) -> Result<DownloadResult> {
        // Flush and sync file
        file.flush().await.map_err(crate::Error::Io)?;
        file.sync_all().await.map_err(crate::Error::Io)?;
        drop(file);

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

    /// Prepare download file (create or open for append if resuming)
    async fn prepare_download_file(&self, file_path: &Path) -> Result<(File, u64)> {
        if file_path.exists() {
            // File exists, open for append and return current size
            let file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(file_path)
                .await
                .map_err(crate::Error::Io)?;

            let metadata = tokio::fs::metadata(file_path)
                .await
                .map_err(crate::Error::Io)?;

            Ok((file, metadata.len()))
        } else {
            // Create new file
            let file = File::create(file_path).await.map_err(crate::Error::Io)?;

            Ok((file, 0))
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
}

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
            max_retries: 2,
        };
        Arc::new(config)
    }

    fn create_test_download_tool() -> Result<DownloadTool> {
        let config = create_test_config();
        let meta_config = crate::client::MetaSearchConfig::default();
        let client = Arc::new(MetaSearchClient::new((*config).clone(), meta_config)?);
        Ok(DownloadTool::new(client, config))
    }

    #[test]
    fn test_download_input_validation() {
        // No DOI or URL should fail
        let empty_input = DownloadInput {
            doi: None,
            url: None,
            filename: None,
            directory: None,
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
    async fn test_prepare_download_file() {
        let tool = create_test_download_tool().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.pdf");

        // Test new file creation
        let (file, start_byte) = tool.prepare_download_file(&file_path).await.unwrap();
        assert_eq!(start_byte, 0);
        drop(file);

        // Write some data
        tokio::fs::write(&file_path, b"test data").await.unwrap();

        // Test resume (file exists)
        let (file, start_byte) = tool.prepare_download_file(&file_path).await.unwrap();
        assert_eq!(start_byte, 9); // "test data".len()
        drop(file);
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
        let meta_config = crate::client::MetaSearchConfig::default();
        let client = Arc::new(MetaSearchClient::new(config.clone(), meta_config).unwrap());
        let tool = DownloadTool::new(client, Arc::new(config));

        // Test that the tool uses the custom directory
        let dir = tool.get_default_download_directory();
        assert_eq!(dir, PathBuf::from("/tmp/test-downloads"));

        // Test file path determination with no override
        let input = DownloadInput {
            doi: Some("10.1038/test".to_string()),
            url: None,
            filename: Some("test.pdf".to_string()),
            directory: None, // No override, should use config default
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
