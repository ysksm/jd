//! Ollama Embedding API client
//!
//! Provides integration with Ollama's local embedding API.
//! Default model: nomic-embed-text (768 dimensions)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::EmbeddingProvider;
use crate::domain::error::{DomainError, DomainResult};

/// Ollama embedding model configurations
pub mod models {
    /// nomic-embed-text - 768 dimensions, fast
    pub const NOMIC_EMBED_TEXT: &str = "nomic-embed-text";
    pub const NOMIC_EMBED_TEXT_DIM: usize = 768;

    /// mxbai-embed-large - 1024 dimensions, higher quality
    pub const MXBAI_EMBED_LARGE: &str = "mxbai-embed-large";
    pub const MXBAI_EMBED_LARGE_DIM: usize = 1024;

    /// snowflake-arctic-embed - 1024 dimensions
    pub const SNOWFLAKE_ARCTIC_EMBED: &str = "snowflake-arctic-embed";
    pub const SNOWFLAKE_ARCTIC_EMBED_DIM: usize = 1024;
}

/// Configuration for Ollama embedding client
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama API endpoint (default: http://localhost:11434)
    pub endpoint: String,
    /// Model to use for embeddings
    pub model: String,
    /// Embedding dimension
    pub dimension: usize,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: models::NOMIC_EMBED_TEXT.to_string(),
            dimension: models::NOMIC_EMBED_TEXT_DIM,
            timeout: Duration::from_secs(120),
        }
    }
}

impl OllamaConfig {
    /// Create a new configuration with the specified model
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let dimension = match model.as_str() {
            models::NOMIC_EMBED_TEXT => models::NOMIC_EMBED_TEXT_DIM,
            models::MXBAI_EMBED_LARGE => models::MXBAI_EMBED_LARGE_DIM,
            models::SNOWFLAKE_ARCTIC_EMBED => models::SNOWFLAKE_ARCTIC_EMBED_DIM,
            _ => models::NOMIC_EMBED_TEXT_DIM, // default
        };
        Self {
            model,
            dimension,
            ..Default::default()
        }
    }

    /// Set custom endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set custom dimension (for custom models)
    pub fn with_dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }
}

/// Ollama Embedding API client
pub struct OllamaEmbeddingClient {
    client: Client,
    config: OllamaConfig,
}

impl OllamaEmbeddingClient {
    /// Create a new Ollama embedding client
    pub fn new(config: OllamaConfig) -> DomainResult<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, config })
    }

    /// Get the configuration
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingClient {
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>> {
        let request = OllamaEmbeddingRequest {
            model: &self.config.model,
            prompt: text,
        };

        let url = format!("{}/api/embeddings", self.config.endpoint);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    DomainError::ExternalService(format!(
                        "Failed to connect to Ollama at {}. Is Ollama running? Start with: ollama serve",
                        self.config.endpoint
                    ))
                } else {
                    DomainError::ExternalService(format!("Failed to send embedding request: {}", e))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status.as_u16() == 404 {
                return Err(DomainError::ExternalService(format!(
                    "Model '{}' not found. Install with: ollama pull {}",
                    self.config.model, self.config.model
                )));
            }

            return Err(DomainError::ExternalService(format!(
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        let response: OllamaEmbeddingResponse = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse embedding response: {}", e))
        })?;

        Ok(response.embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>> {
        // Ollama doesn't support batch embedding, so we process one by one
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            let embedding = self.embed(text).await?;
            embeddings.push(embedding);
        }
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }
}

// Ollama API request/response types
#[derive(Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.model, models::NOMIC_EMBED_TEXT);
        assert_eq!(config.dimension, models::NOMIC_EMBED_TEXT_DIM);
        assert_eq!(config.endpoint, "http://localhost:11434");
    }

    #[test]
    fn test_config_builder() {
        let config = OllamaConfig::new("mxbai-embed-large")
            .with_endpoint("http://192.168.1.100:11434")
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.model, "mxbai-embed-large");
        assert_eq!(config.dimension, models::MXBAI_EMBED_LARGE_DIM);
        assert_eq!(config.endpoint, "http://192.168.1.100:11434");
    }
}
