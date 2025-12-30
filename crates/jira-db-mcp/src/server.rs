//! MCP Server implementation for JIRA database
//!
//! This module provides the main server loop that handles MCP requests
//! over different transports (stdio, HTTP/SSE).

use std::sync::Arc;

use anyhow::Result;
use duckdb::Connection;
use std::sync::Mutex;

use jira_db_core::Database;

use crate::config::McpConfig;
use crate::handlers::RequestHandler;
use crate::protocol::ProtocolError;
use crate::tools::ToolRegistry;
use crate::transport::{StdioTransport, Transport};

/// MCP Server for JIRA Database
///
/// Handles MCP protocol communication over various transports.
pub struct McpServer {
    db_conn: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    config: McpConfig,
}

impl McpServer {
    /// Create a new MCP server instance
    pub fn new(config: McpConfig) -> Result<Self> {
        let db = Database::new(&config.database_path)?;

        Ok(Self {
            db_conn: db.connection(),
            config,
        })
    }

    /// Run the server over stdio transport
    pub async fn run_stdio(self) -> Result<()> {
        tracing::info!("Starting MCP server over stdio");

        let mut transport = StdioTransport::new();
        let tool_registry = ToolRegistry::new(self.db_conn.clone());
        let mut handler = RequestHandler::new(tool_registry);

        loop {
            match transport.read_request().await {
                Ok(Some(request)) => {
                    let id = request.id.clone();

                    match handler.handle(request).await {
                        Ok(Some(response)) => {
                            if let Err(e) = transport.send_response(response).await {
                                tracing::error!("Failed to send response: {}", e);
                            }
                        }
                        Ok(None) => {
                            // Notification - no response needed
                        }
                        Err(e) => {
                            let error_response = e.to_error_response(id);
                            if let Ok(value) = serde_json::to_value(&error_response) {
                                if let Err(send_err) = transport.send_response(value).await {
                                    tracing::error!("Failed to send error response: {}", send_err);
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // EOF - client closed connection
                    tracing::info!("Client disconnected");
                    break;
                }
                Err(ProtocolError::JsonParse(e)) => {
                    tracing::warn!("Failed to parse request: {}", e);
                    let error_response =
                        ProtocolError::JsonParse(e).to_error_response(None);
                    if let Ok(value) = serde_json::to_value(&error_response) {
                        if let Err(send_err) = transport.send_response(value).await {
                            tracing::error!("Failed to send error response: {}", send_err);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Transport error: {}", e);
                    break;
                }
            }
        }

        transport.close().await?;
        tracing::info!("MCP server stopped");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_server_creation_fails_without_db() {
        let config = McpConfig {
            database_path: PathBuf::from("/nonexistent/path/to/db"),
            ..McpConfig::default_config()
        };

        let result = McpServer::new(config);
        assert!(result.is_err());
    }
}
