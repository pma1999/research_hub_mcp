pub mod client;
pub mod config;
pub mod error;
pub mod resilience;
pub mod server;
pub mod service;
pub mod tools;

pub use client::{ResearchClient, Doi, PaperMetadata, MetaSearchClient, MetaSearchConfig, MetaSearchResult};
pub use config::{Config, ConfigOverrides};
pub use error::{Error, Result};
pub use resilience::{CircuitBreaker, RetryConfig, RetryPolicy, TimeoutConfig, TimeoutExt};
pub use resilience::health::HealthCheckManager;
pub use server::Server;
pub use service::{DaemonConfig, DaemonService, HealthCheck, PidFile, SignalHandler};
pub use tools::{SearchTool, DownloadTool, MetadataExtractor};

