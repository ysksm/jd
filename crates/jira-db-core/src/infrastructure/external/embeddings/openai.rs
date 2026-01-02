//! OpenAI Embedding API client
//!
//! Provides integration with OpenAI's text embedding API.
//! Also supports OpenAI-compatible APIs like LM Studio, LocalAI, etc.
//! Default model: text-embedding-3-small (1536 dimensions)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::EmbeddingProvider;
use crate::domain::error::{DomainError, DomainResult};

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

    /// nomic-embed-text - 768 dimensions (common for local models)
    pub const NOMIC_EMBED_TEXT_DIM: usize = 768;
}

/// Configuration for OpenAI embedding client
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// OpenAI API key (optional for local servers like LM Studio)
    pub api_key: Option<String>,
    /// Model to use for embeddings
    pub model: String,
    /// Embedding dimension (None = auto-detect from first response)
    pub dimension: Option<usize>,
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
            api_key: None,
            model: models::TEXT_EMBEDDING_3_SMALL.to_string(),
            dimension: Some(models::TEXT_EMBEDDING_3_SMALL_DIM),
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
            api_key: Some(api_key.into()),
            ..Default::default()
        }
    }

    /// Create a new configuration for a local server (no API key required)
    pub fn local(api_base: impl Into<String>) -> Self {
        Self {
            api_key: None,
            api_base: api_base.into(),
            dimension: None, // Auto-detect
            ..Default::default()
        }
    }

    /// Use text-embedding-3-small model (default)
    pub fn with_small_model(mut self) -> Self {
        self.model = models::TEXT_EMBEDDING_3_SMALL.to_string();
        self.dimension = Some(models::TEXT_EMBEDDING_3_SMALL_DIM);
        self
    }

    /// Use text-embedding-3-large model
    pub fn with_large_model(mut self) -> Self {
        self.model = models::TEXT_EMBEDDING_3_LARGE.to_string();
        self.dimension = Some(models::TEXT_EMBEDDING_3_LARGE_DIM);
        self
    }

    /// Set custom model with dimension
    pub fn with_model(mut self, model: impl Into<String>, dimension: Option<usize>) -> Self {
        self.model = model.into();
        self.dimension = dimension;
        self
    }

    /// Set custom API base URL (for LM Studio, LocalAI, or other compatible APIs)
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
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

    /// Check if this is configured for a local server
    pub fn is_local(&self) -> bool {
        self.api_base.contains("localhost") || self.api_base.contains("127.0.0.1")
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
/// Also works with OpenAI-compatible APIs like LM Studio, LocalAI, etc.
pub struct OpenAIEmbeddingClient {
    client: Client,
    config: EmbeddingConfig,
    /// Cached dimension (auto-detected from first response if not specified)
    detected_dimension: std::sync::RwLock<Option<usize>>,
}

impl OpenAIEmbeddingClient {
    /// Create a new OpenAI embedding client
    pub fn new(config: EmbeddingConfig) -> DomainResult<Self> {
        // Only require API key for non-local servers
        if config.api_key.is_none() && !config.is_local() {
            return Err(DomainError::Configuration(
                "OpenAI API key is required for non-local servers".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            detected_dimension: std::sync::RwLock::new(None),
        })
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

        // Build request with optional authorization
        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add authorization header if API key is present
        if let Some(ref api_key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.json(&request).send().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to send embedding request: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "Embedding API error ({}): {}",
                status, error_text
            )));
        }

        let response: EmbeddingResponse = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse embedding response: {}", e))
        })?;

        // Auto-detect dimension from first response
        if let Some(first) = response.data.first() {
            let dim = first.embedding.len();
            if let Ok(mut detected) = self.detected_dimension.write() {
                if detected.is_none() {
                    *detected = Some(dim);
                }
            }
        }

        let token_count = response
            .usage
            .as_ref()
            .map(|u| u.prompt_tokens / texts.len())
            .unwrap_or(0);

        let results: Vec<EmbeddingResult> = response
            .data
            .into_iter()
            .map(|d| EmbeddingResult {
                embedding: d.embedding,
                token_count,
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
        // Return configured dimension, detected dimension, or default
        self.config
            .dimension
            .or_else(|| self.detected_dimension.read().ok().and_then(|d| *d))
            .unwrap_or(models::TEXT_EMBEDDING_3_SMALL_DIM)
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
    /// Usage info (optional, some local servers don't return this)
    usage: Option<EmbeddingUsage>,
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
        assert_eq!(config.dimension, Some(models::TEXT_EMBEDDING_3_SMALL_DIM));
    }

    #[test]
    fn test_config_builder() {
        let config = EmbeddingConfig::new("test-key")
            .with_large_model()
            .with_batch_size(50);

        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, models::TEXT_EMBEDDING_3_LARGE);
        assert_eq!(config.dimension, Some(models::TEXT_EMBEDDING_3_LARGE_DIM));
        assert_eq!(config.batch_size, 50);
    }
}
