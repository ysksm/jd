//! OpenAI Embedding API client
//!
//! Provides integration with OpenAI's text embedding API.
//! Default model: text-embedding-3-small (1536 dimensions)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::error::{DomainError, DomainResult};
use super::EmbeddingProvider;

/// OpenAI embedding model configurations
#[allow(dead_code)]
pub mod models {
    /// text-embedding-3-small - 1536 dimensions, cost-effective
    pub const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";
    pub const TEXT_EMBEDDING_3_SMALL_DIM: usize = 1536;

    /// text-embedding-3-large - 3072 dimensions, higher quality
    pub const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
    pub const TEXT_EMBEDDING_3_LARGE_DIM: usize = 3072;

    /// text-embedding-ada-002 - 1536 dimensions, legacy model
    pub const TEXT_EMBEDDING_ADA_002: &str = "text-embedding-ada-002";
    pub const TEXT_EMBEDDING_ADA_002_DIM: usize = 1536;
}

/// Configuration for OpenAI embedding client
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// OpenAI API key
    pub api_key: String,
    /// Model to use for embeddings
    pub model: String,
    /// Embedding dimension
    pub dimension: usize,
    /// API endpoint (default: https://api.openai.com/v1)
    pub api_base: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum batch size for embedding requests
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: models::TEXT_EMBEDDING_3_SMALL.to_string(),
            dimension: models::TEXT_EMBEDDING_3_SMALL_DIM,
            api_base: "https://api.openai.com/v1".to_string(),
            timeout: Duration::from_secs(60),
            batch_size: 100,
        }
    }
}

impl EmbeddingConfig {
    /// Create a new configuration with the specified API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Use text-embedding-3-small model (default)
    pub fn with_small_model(mut self) -> Self {
        self.model = models::TEXT_EMBEDDING_3_SMALL.to_string();
        self.dimension = models::TEXT_EMBEDDING_3_SMALL_DIM;
        self
    }

    /// Use text-embedding-3-large model
    pub fn with_large_model(mut self) -> Self {
        self.model = models::TEXT_EMBEDDING_3_LARGE.to_string();
        self.dimension = models::TEXT_EMBEDDING_3_LARGE_DIM;
        self
    }

    /// Set custom API base URL (for Azure OpenAI or other compatible APIs)
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set batch size for embedding requests
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

/// Result from embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    /// The generated embedding vector
    pub embedding: Vec<f32>,
    /// Token count used for the input
    pub token_count: usize,
}

/// OpenAI Embedding API client
pub struct OpenAIEmbeddingClient {
    client: Client,
    config: EmbeddingConfig,
}

impl OpenAIEmbeddingClient {
    /// Create a new OpenAI embedding client
    pub fn new(config: EmbeddingConfig) -> DomainResult<Self> {
        if config.api_key.is_empty() {
            return Err(DomainError::Configuration(
                "OpenAI API key is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| DomainError::ExternalService(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Create embeddings for a batch of texts
    pub async fn create_embeddings(&self, texts: &[&str]) -> DomainResult<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = EmbeddingRequest {
            model: &self.config.model,
            input: texts.to_vec(),
            encoding_format: Some("float"),
        };

        let url = format!("{}/embeddings", self.config.api_base);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to send embedding request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to parse embedding response: {}", e)))?;

        let results: Vec<EmbeddingResult> = response
            .data
            .into_iter()
            .map(|d| EmbeddingResult {
                embedding: d.embedding,
                token_count: response.usage.prompt_tokens / texts.len(),
            })
            .collect();

        Ok(results)
    }

    /// Get the configuration
    pub fn config(&self) -> &EmbeddingConfig {
        &self.config
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingClient {
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>> {
        let results = self.create_embeddings(&[text]).await?;
        results
            .into_iter()
            .next()
            .map(|r| r.embedding)
            .ok_or_else(|| DomainError::ExternalService("No embedding returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::with_capacity(texts.len());

        // Process in batches
        for chunk in texts.chunks(self.config.batch_size) {
            let results = self.create_embeddings(chunk).await?;
            all_embeddings.extend(results.into_iter().map(|r| r.embedding));
        }

        Ok(all_embeddings)
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }
}

// OpenAI API request/response types
#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<&'a str>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: EmbeddingUsage,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    prompt_tokens: usize,
    #[allow(dead_code)]
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.model, models::TEXT_EMBEDDING_3_SMALL);
        assert_eq!(config.dimension, models::TEXT_EMBEDDING_3_SMALL_DIM);
    }

    #[test]
    fn test_config_builder() {
        let config = EmbeddingConfig::new("test-key")
            .with_large_model()
            .with_batch_size(50);

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, models::TEXT_EMBEDDING_3_LARGE);
        assert_eq!(config.dimension, models::TEXT_EMBEDDING_3_LARGE_DIM);
        assert_eq!(config.batch_size, 50);
    }
}
