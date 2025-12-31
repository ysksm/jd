//! Configuration for the MCP server

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Path to the DuckDB database file
    pub database_path: PathBuf,

    /// HTTP server configuration
    #[serde(default)]
    pub http: HttpConfig,

    /// Embedding configuration for vector search
    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Whether to enable HTTP server
    #[serde(default)]
    pub enabled: bool,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_port(),
            host: default_host(),
        }
    }
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

/// Embedding provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Embedding provider to use
    #[serde(default)]
    pub provider: EmbeddingProvider,

    /// API key for the embedding service
    pub api_key: Option<String>,

    /// Model name to use
    #[serde(default = "default_embedding_model")]
    pub model: String,

    /// API endpoint (for custom providers)
    pub endpoint: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::default(),
            api_key: None,
            model: default_embedding_model(),
            endpoint: None,
        }
    }
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

/// Supported embedding providers
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    #[default]
    OpenAI,
    AzureOpenAI,
    Ollama,
    Cohere,
    VoyageAI,
}

impl McpConfig {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Get default config file path
    pub fn default_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("jira-db-mcp").join("config.json")
        } else {
            PathBuf::from("jira-db-mcp.json")
        }
    }

    /// Create a default configuration
    pub fn default_config() -> Self {
        Self {
            database_path: PathBuf::from("./data/jira.duckdb"),
            http: HttpConfig::default(),
            embedding: EmbeddingConfig::default(),
        }
    }

    /// Save configuration to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpConfig::default_config();
        assert_eq!(config.http.port, 3000);
        assert_eq!(config.http.host, "127.0.0.1");
        assert!(!config.http.enabled);
        assert_eq!(config.embedding.provider, EmbeddingProvider::OpenAI);
    }

    #[test]
    fn test_config_serialization() {
        let config = McpConfig::default_config();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: McpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.http.port, config.http.port);
    }
}
