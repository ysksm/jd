//! Cohere Embedding API client
//!
//! Provides integration with Cohere's embedding API.
//! Default model: embed-multilingual-v3.0 (1024 dimensions)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::EmbeddingProvider;
use crate::domain::error::{DomainError, DomainResult};

/// Cohere embedding model configurations
#[allow(dead_code)]
pub mod models {
    /// embed-multilingual-v3.0 - 1024 dimensions, 100+ languages
    pub const EMBED_MULTILINGUAL_V3: &str = "embed-multilingual-v3.0";
    pub const EMBED_MULTILINGUAL_V3_DIM: usize = 1024;

    /// embed-english-v3.0 - 1024 dimensions, English optimized
    pub const EMBED_ENGLISH_V3: &str = "embed-english-v3.0";
    pub const EMBED_ENGLISH_V3_DIM: usize = 1024;

    /// embed-multilingual-light-v3.0 - 384 dimensions, faster
    pub const EMBED_MULTILINGUAL_LIGHT_V3: &str = "embed-multilingual-light-v3.0";
    pub const EMBED_MULTILINGUAL_LIGHT_V3_DIM: usize = 384;

    /// embed-english-light-v3.0 - 384 dimensions, faster
    pub const EMBED_ENGLISH_LIGHT_V3: &str = "embed-english-light-v3.0";
    pub const EMBED_ENGLISH_LIGHT_V3_DIM: usize = 384;
}

/// Input type for Cohere embeddings
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    /// For search queries
    SearchQuery,
    /// For documents to be searched
    SearchDocument,
    /// For classification tasks
    Classification,
    /// For clustering tasks
    Clustering,
}

/// Configuration for Cohere embedding client
#[derive(Debug, Clone)]
pub struct CohereConfig {
    /// Cohere API key
    pub api_key: String,
    /// Model to use for embeddings
    pub model: String,
    /// Embedding dimension
    pub dimension: usize,
    /// Input type (affects embedding optimization)
    pub input_type: InputType,
    /// API endpoint
    pub api_base: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum batch size
    pub batch_size: usize,
}

impl Default for CohereConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: models::EMBED_MULTILINGUAL_V3.to_string(),
            dimension: models::EMBED_MULTILINGUAL_V3_DIM,
            input_type: InputType::SearchDocument,
            api_base: "https://api.cohere.ai/v1".to_string(),
            timeout: Duration::from_secs(60),
            batch_size: 96, // Cohere limit
        }
    }
}

impl CohereConfig {
    /// Create a new configuration with the specified API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Use multilingual model (default)
    pub fn with_multilingual_model(mut self) -> Self {
        self.model = models::EMBED_MULTILINGUAL_V3.to_string();
        self.dimension = models::EMBED_MULTILINGUAL_V3_DIM;
        self
    }

    /// Use English-optimized model
    pub fn with_english_model(mut self) -> Self {
        self.model = models::EMBED_ENGLISH_V3.to_string();
        self.dimension = models::EMBED_ENGLISH_V3_DIM;
        self
    }

    /// Use light (faster) multilingual model
    pub fn with_light_multilingual_model(mut self) -> Self {
        self.model = models::EMBED_MULTILINGUAL_LIGHT_V3.to_string();
        self.dimension = models::EMBED_MULTILINGUAL_LIGHT_V3_DIM;
        self
    }

    /// Set custom model
    pub fn with_model(mut self, model: impl Into<String>, dimension: usize) -> Self {
        self.model = model.into();
        self.dimension = dimension;
        self
    }

    /// Set input type for search queries (use when embedding queries)
    pub fn for_search_query(mut self) -> Self {
        self.input_type = InputType::SearchQuery;
        self
    }

    /// Set input type for search documents (use when embedding documents)
    pub fn for_search_document(mut self) -> Self {
        self.input_type = InputType::SearchDocument;
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.min(96); // Cohere limit
        self
    }
}

/// Cohere Embedding API client
pub struct CohereEmbeddingClient {
    client: Client,
    config: CohereConfig,
}

impl CohereEmbeddingClient {
    /// Create a new Cohere embedding client
    pub fn new(config: CohereConfig) -> DomainResult<Self> {
        if config.api_key.is_empty() {
            return Err(DomainError::Configuration(
                "Cohere API key is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, config })
    }

    /// Get the configuration
    pub fn config(&self) -> &CohereConfig {
        &self.config
    }

    /// Create embeddings for a batch of texts
    async fn create_embeddings(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = CohereEmbeddingRequest {
            model: &self.config.model,
            texts: texts.to_vec(),
            input_type: self.config.input_type,
            truncate: Some("END"),
        };

        let url = format!("{}/embed", self.config.api_base);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to send embedding request: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "Cohere API error ({}): {}",
                status, error_text
            )));
        }

        let response: CohereEmbeddingResponse = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse embedding response: {}", e))
        })?;

        Ok(response.embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for CohereEmbeddingClient {
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>> {
        let embeddings = self.create_embeddings(&[text]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::ExternalService("No embedding returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::with_capacity(texts.len());

        // Process in batches
        for chunk in texts.chunks(self.config.batch_size) {
            let embeddings = self.create_embeddings(chunk).await?;
            all_embeddings.extend(embeddings);
        }

        Ok(all_embeddings)
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn provider_name(&self) -> &str {
        "cohere"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

// Cohere API request/response types
#[derive(Serialize)]
struct CohereEmbeddingRequest<'a> {
    model: &'a str,
    texts: Vec<&'a str>,
    input_type: InputType,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncate: Option<&'a str>,
}

#[derive(Deserialize)]
struct CohereEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CohereConfig::default();
        assert_eq!(config.model, models::EMBED_MULTILINGUAL_V3);
        assert_eq!(config.dimension, models::EMBED_MULTILINGUAL_V3_DIM);
    }

    #[test]
    fn test_config_builder() {
        let config = CohereConfig::new("test-key")
            .with_english_model()
            .for_search_query();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, models::EMBED_ENGLISH_V3);
        assert!(matches!(config.input_type, InputType::SearchQuery));
    }
}
