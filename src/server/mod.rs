pub mod handler;
pub mod transport;

use crate::{Config, Error, Result};
use std::sync::Arc;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub use handler::SciHubServerHandler;

pub struct Server {
    config: Arc<Config>,
    cancellation_token: CancellationToken,
}

impl Server {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            cancellation_token: CancellationToken::new(),
        }
    }

    #[must_use]
    pub fn new_with_arc(config: Arc<Config>) -> Self {
        Self {
            config,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting MCP server infrastructure");

        // Create server handler
        let mut handler = SciHubServerHandler::new(Arc::clone(&self.config))?;

        // Validate transport setup
        transport::validate_stdio_transport().map_err(|e| {
            Error::Service(format!("Transport validation failed: {e}"))
        })?;

        // Initialize handler
        let server_name = handler.initialize().await?;
        info!("Server initialized: {}", server_name);

        // Setup signal handlers
        let shutdown_token = self.cancellation_token.clone();
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to setup SIGTERM handler");
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to setup SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown");
                }
            }

            shutdown_token.cancel();
        });

        // Run server lifecycle
        info!("Server running - waiting for shutdown signal");
        
        tokio::select! {
            _ = self.health_check_loop(&handler) => {
                info!("Health check completed");
            }
            () = self.cancellation_token.cancelled() => {
                info!("Shutdown signal received, stopping server");
            }
        }

        // Graceful shutdown with timeout
        let shutdown_timeout = tokio::time::Duration::from_secs(self.config.server.graceful_shutdown_timeout_secs);
        if tokio::time::timeout(shutdown_timeout, self.graceful_shutdown()).await.is_err() {
            warn!("Graceful shutdown timeout exceeded, forcing shutdown");
        }

        info!("Server shutdown complete");
        Ok(())
    }

    async fn health_check_loop(&self, handler: &SciHubServerHandler) -> Result<()> {
        let interval_secs = self.config.server.health_check_interval_secs;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = handler.ping().await {
                error!("Health check failed: {}", e);
                return Err(e);
            }
            
            if self.cancellation_token.is_cancelled() {
                break;
            }
        }
        
        Ok(())
    }

    async fn graceful_shutdown(&self) -> Result<()> {
        info!("Performing graceful shutdown");
        
        // Here we would:
        // 1. Stop accepting new requests
        // 2. Wait for existing requests to complete
        // 3. Close connections cleanly
        // 4. Save any necessary state
        
        // For now, just a placeholder
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        info!("Graceful shutdown completed");
        Ok(())
    }

    pub async fn shutdown(&self) {
        warn!("Initiating server shutdown");
        self.cancellation_token.cancel();
        
        // Give a moment for cleanup to begin
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// Check if the server has been requested to shutdown
    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Get the server configuration
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = Config::default();
        let server = Server::new(config);
        assert!(!server.cancellation_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_server_shutdown() {
        let config = Config::default();
        let server = Server::new(config);
        
        server.shutdown().await;
        assert!(server.cancellation_token.is_cancelled());
    }
}