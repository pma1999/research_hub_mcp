//! Rust Research MCP - Academic Paper Search and Metadata Extraction
//!
//! This crate provides a Model Context Protocol (MCP) server for searching and downloading
//! academic papers from multiple sources including `arXiv`, `Semantic Scholar`, `CrossRef`, and more.

pub mod client;
pub mod config;
pub mod error;
pub mod resilience;
pub mod server;
pub mod service;
pub mod tools;

pub use client::{Doi, MetaSearchClient, MetaSearchConfig, MetaSearchResult, PaperMetadata};
pub use config::{Config, ConfigOverrides};
pub use error::{Error, Result};
pub use resilience::health::HealthCheckManager;
pub use resilience::{CircuitBreaker, RetryConfig, RetryPolicy, TimeoutConfig, TimeoutExt};
pub use server::Server;
pub use service::{DaemonConfig, DaemonService, HealthCheck, PidFile, SignalHandler};
pub use tools::{DownloadTool, MetadataExtractor, SearchTool, CodeSearchTool, BibliographyTool};
