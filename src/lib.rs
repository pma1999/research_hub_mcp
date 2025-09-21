//! Rust Research MCP - Academic Paper Search and Metadata Extraction
//!
//! This crate provides a Model Context Protocol (MCP) server for searching and downloading
//! academic papers from multiple sources including `arXiv`, `Semantic Scholar`, `CrossRef`, and more.

// pub mod adapters;
pub mod client;
pub mod config;
// pub mod di;
pub mod error;
// pub mod ports;
// pub mod repositories;
pub mod resilience;
pub mod server;
pub mod service;
pub mod services;
pub mod tools;

// pub use adapters::{
//     MetaSearchAdapter, MultiProviderAdapter, PaperDownloadAdapter, PdfMetadataAdapter,
// };
pub use client::{Doi, MetaSearchClient, MetaSearchConfig, MetaSearchResult, PaperMetadata};
pub use config::{Config, ConfigOverrides};
// pub use di::{ServiceContainer, ServiceScope};
pub use error::{Error, Result};
// pub use ports::{DownloadServicePort, MetadataServicePort, ProviderServicePort, SearchServicePort};
// pub use repositories::{
//     CacheRepository, ConfigRepository, InMemoryCacheRepository, InMemoryConfigRepository,
//     InMemoryPaperRepository, PaperQuery, PaperRepository, Repository, RepositoryError,
//     RepositoryResult, RepositoryStats,
// };
pub use resilience::health::HealthCheckManager;
pub use resilience::{CircuitBreaker, RetryConfig, RetryPolicy, TimeoutConfig, TimeoutExt};
pub use server::Server;
pub use service::{DaemonConfig, DaemonService, HealthCheck, PidFile, SignalHandler};
pub use tools::{
    BibliographyTool, CategorizeTool, CodeSearchTool, DownloadTool, MetadataExtractor, SearchTool,
};
