use std::time::Duration;
use thiserror::Error;

/// Comprehensive error categorization for resilience framework
#[derive(Error, Debug)]
pub enum Error {
    // Configuration errors (permanent failures)
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    // I/O errors (potentially transient)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Serialization errors (usually permanent)
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    // Network errors (transient - should retry)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Network timeout after {timeout:?}: {message}")]
    NetworkTimeout { timeout: Duration, message: String },

    #[error("Connection refused: {endpoint}")]
    ConnectionRefused { endpoint: String },

    #[error("DNS resolution failed: {hostname}")]
    DnsFailure { hostname: String },

    // Service-specific errors
    #[error("MCP protocol error: {0}")]
    Mcp(String),

    #[error("Sci-Hub service error: {code} - {message}")]
    SciHub { code: u16, message: String },

    #[error("Rate limit exceeded: retry after {retry_after:?}")]
    RateLimitExceeded { retry_after: Duration },

    // Client errors (permanent - don't retry)
    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authorization denied: {resource}")]
    AuthorizationDenied { resource: String },

    // Server errors (transient - should retry)
    #[error("Service temporarily unavailable: {service} - {reason}")]
    ServiceUnavailable { service: String, reason: String },

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Service overloaded: {service}")]
    ServiceOverloaded { service: String },

    // Circuit breaker errors
    #[error("Circuit breaker open for service: {service}")]
    CircuitBreakerOpen { service: String },

    #[error("Circuit breaker half-open, limited requests allowed")]
    CircuitBreakerHalfOpen,

    // Resource errors
    #[error("Resource exhausted: {resource} - {current}/{limit}")]
    ResourceExhausted {
        resource: String,
        current: u64,
        limit: u64,
    },

    #[error("Timeout error: operation timed out after {timeout:?}")]
    Timeout { timeout: Duration },

    // Cache errors
    #[error("Cache error: {operation} failed - {reason}")]
    Cache { operation: String, reason: String },

    // Parse errors
    #[error("Parse error in {context}: {message}")]
    Parse { context: String, message: String },

    // General service error
    #[error("Service error: {0}")]
    Service(String),

    // Provider errors
    #[error("Provider error: {0}")]
    Provider(String),
}

/// Error categorization for retry strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// Permanent errors - should not retry
    Permanent,
    /// Transient errors - safe to retry
    Transient,
    /// Rate limited - retry with backoff
    RateLimited,
    /// Circuit breaker triggered - stop retrying temporarily
    CircuitBreaker,
}

impl Error {
    /// Categorize error for retry logic
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Permanent errors - don't retry
            Error::Config(_)
            | Error::InvalidInput { .. }
            | Error::AuthenticationFailed(_)
            | Error::AuthorizationDenied { .. }
            | Error::Parse { .. }
            | Error::Serde(_) => ErrorCategory::Permanent,

            // Rate limited - retry with backoff
            Error::RateLimitExceeded { .. } => ErrorCategory::RateLimited,

            // Circuit breaker errors
            Error::CircuitBreakerOpen { .. } | Error::CircuitBreakerHalfOpen => {
                ErrorCategory::CircuitBreaker
            }

            // Transient errors - retry with exponential backoff
            Error::Http(_)
            | Error::NetworkTimeout { .. }
            | Error::ConnectionRefused { .. }
            | Error::DnsFailure { .. }
            | Error::ServiceUnavailable { .. }
            | Error::InternalServerError(_)
            | Error::ServiceOverloaded { .. }
            | Error::Timeout { .. }
            | Error::Io(_) => ErrorCategory::Transient,

            // Service specific - depends on context
            Error::SciHub { code, .. } => {
                match *code {
                    // Rate limiting (handle first to avoid unreachable pattern)
                    429 => ErrorCategory::RateLimited,
                    // 4xx client errors are permanent
                    400..=499 => ErrorCategory::Permanent,
                    // 5xx server errors are transient
                    500..=599 => ErrorCategory::Transient,
                    // Other codes treated as transient
                    _ => ErrorCategory::Transient,
                }
            }

            // Provider errors - categorize based on the error type
            Error::Provider(_) => ErrorCategory::Transient,

            // Default to transient for unknown errors
            _ => ErrorCategory::Transient,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Transient | ErrorCategory::RateLimited
        )
    }

    /// Get suggested retry delay for rate limited errors
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimitExceeded { retry_after } => Some(*retry_after),
            _ => None,
        }
    }

    /// Check if error indicates a need for circuit breaker
    pub fn should_trigger_circuit_breaker(&self) -> bool {
        matches!(
            self,
            Error::ServiceUnavailable { .. }
                | Error::InternalServerError(_)
                | Error::ServiceOverloaded { .. }
                | Error::NetworkTimeout { .. }
                | Error::ConnectionRefused { .. }
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Provider error conversion
impl From<crate::client::providers::ProviderError> for Error {
    fn from(err: crate::client::providers::ProviderError) -> Self {
        match err {
            crate::client::providers::ProviderError::Network(msg) => {
                Error::Provider(format!("Network error: {}", msg))
            }
            crate::client::providers::ProviderError::Parse(msg) => Error::Parse {
                context: "provider".to_string(),
                message: msg,
            },
            crate::client::providers::ProviderError::RateLimit => Error::RateLimitExceeded {
                retry_after: Duration::from_secs(60),
            },
            crate::client::providers::ProviderError::Auth(msg) => Error::AuthenticationFailed(msg),
            crate::client::providers::ProviderError::InvalidQuery(msg) => Error::InvalidInput {
                field: "query".to_string(),
                reason: msg,
            },
            crate::client::providers::ProviderError::ServiceUnavailable(msg) => {
                Error::ServiceUnavailable {
                    service: "provider".to_string(),
                    reason: msg,
                }
            }
            crate::client::providers::ProviderError::Timeout => Error::Timeout {
                timeout: Duration::from_secs(30),
            },
            crate::client::providers::ProviderError::Other(msg) => Error::Provider(msg),
        }
    }
}
