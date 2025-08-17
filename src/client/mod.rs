pub mod sci_hub;
pub mod mirror;
pub mod rate_limiter;
pub mod providers;
pub mod meta_search;

pub use sci_hub::{ResearchClient, ResearchResponse};
pub use meta_search::{MetaSearchClient, MetaSearchConfig, MetaSearchResult};
pub use mirror::{Mirror, MirrorHealth, MirrorManager};
pub use rate_limiter::RateLimiter;

use crate::Result;
use std::time::Duration;

/// HTTP client configuration for research source integration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout duration
    pub timeout: Duration,
    /// Connection timeout duration
    pub connect_timeout: Duration,
    /// Maximum redirects to follow
    pub max_redirects: u32,
    /// User agent string
    pub user_agent: String,
    /// Proxy URL (optional)
    pub proxy: Option<String>,
    /// Whether to verify SSL certificates
    pub danger_accept_invalid_certs: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_redirects: 10,
            user_agent: "rust-research-mcp/0.2.0 (Academic Research Tool)".to_string(),
            proxy: None,
            danger_accept_invalid_certs: false,
        }
    }
}

/// DOI (Digital Object Identifier) wrapper for type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Doi(String);

impl Doi {
    /// Create a new DOI from a string, validating the format
    pub fn new(doi: &str) -> Result<Self> {
        let cleaned = doi.trim().trim_start_matches("doi:").trim_start_matches("https://doi.org/");
        
        if cleaned.is_empty() {
            return Err(crate::Error::InvalidInput {
                field: "doi".to_string(),
                reason: "DOI cannot be empty".to_string(),
            });
        }
        
        // Basic DOI format validation (simplified)
        if !cleaned.contains('/') {
            return Err(crate::Error::InvalidInput {
                field: "doi".to_string(),
                reason: "DOI must contain a '/' character".to_string(),
            });
        }
        
        Ok(Self(cleaned.to_string()))
    }
    
    /// Get the DOI string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Convert to a URL-safe format
    #[must_use]
    pub fn url_encoded(&self) -> String {
        urlencoding::encode(&self.0).to_string()
    }
}

impl std::fmt::Display for Doi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Doi {
    type Err = crate::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// Paper metadata extracted from research sources
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PaperMetadata {
    /// Digital Object Identifier
    pub doi: String,
    /// Paper title
    pub title: Option<String>,
    /// Authors
    pub authors: Vec<String>,
    /// Journal name
    pub journal: Option<String>,
    /// Publication year
    pub year: Option<u32>,
    /// Abstract
    pub abstract_text: Option<String>,
    /// Download URL for the PDF
    pub pdf_url: Option<String>,
    /// File size in bytes (if available)
    pub file_size: Option<u64>,
}

impl PaperMetadata {
    /// Create new paper metadata with just a DOI
    #[must_use]
    pub const fn new(doi: String) -> Self {
        Self {
            doi,
            title: None,
            authors: Vec::new(),
            journal: None,
            year: None,
            abstract_text: None,
            pdf_url: None,
            file_size: None,
        }
    }
}