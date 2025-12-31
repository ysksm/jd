//! Embedding providers for vector search
//!
//! This module provides integration with embedding providers like OpenAI, Ollama, and Cohere
//! for generating vector embeddings from text.

mod cohere;
mod ollama;
mod openai;

pub use cohere::{CohereConfig, CohereEmbeddingClient};
pub use ollama::{OllamaConfig, OllamaEmbeddingClient};
pub use openai::{EmbeddingConfig, EmbeddingResult, OpenAIEmbeddingClient};

use async_trait::async_trait;
use crate::domain::error::{DomainError, DomainResult};

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for a single text
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>>;

    /// Generate embeddings for multiple texts (batched)
    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>>;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;
}

/// Implement EmbeddingProvider for Box<dyn EmbeddingProvider> to support dynamic dispatch
#[async_trait]
impl EmbeddingProvider for Box<dyn EmbeddingProvider> {
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>> {
        self.as_ref().embed(text).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>> {
        self.as_ref().embed_batch(texts).await
    }

    fn dimension(&self) -> usize {
        self.as_ref().dimension()
    }
}

/// Supported embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbeddingProviderType {
    /// OpenAI API (requires API key)
    #[default]
    OpenAI,
    /// Ollama (local, free)
    Ollama,
    /// Cohere API (requires API key)
    Cohere,
}

impl std::fmt::Display for EmbeddingProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingProviderType::OpenAI => write!(f, "openai"),
            EmbeddingProviderType::Ollama => write!(f, "ollama"),
            EmbeddingProviderType::Cohere => write!(f, "cohere"),
        }
    }
}

impl std::str::FromStr for EmbeddingProviderType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(EmbeddingProviderType::OpenAI),
            "ollama" => Ok(EmbeddingProviderType::Ollama),
            "cohere" => Ok(EmbeddingProviderType::Cohere),
            _ => Err(DomainError::Configuration(format!(
                "Unknown embedding provider: {}. Supported: openai, ollama, cohere",
                s
            ))),
        }
    }
}

/// Configuration for creating an embedding provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider type
    pub provider: EmbeddingProviderType,
    /// API key (for OpenAI, Cohere)
    pub api_key: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Endpoint URL (for Ollama)
    pub endpoint: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProviderType::OpenAI,
            api_key: None,
            model: None,
            endpoint: None,
        }
    }
}

impl ProviderConfig {
    /// Create OpenAI provider config
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProviderType::OpenAI,
            api_key: Some(api_key.into()),
            model: None,
            endpoint: None,
        }
    }

    /// Create Ollama provider config
    pub fn ollama() -> Self {
        Self {
            provider: EmbeddingProviderType::Ollama,
            api_key: None,
            model: None,
            endpoint: None,
        }
    }

    /// Create Cohere provider config
    pub fn cohere(api_key: impl Into<String>) -> Self {
        Self {
            provider: EmbeddingProviderType::Cohere,
            api_key: Some(api_key.into()),
            model: None,
            endpoint: None,
        }
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }
}

/// Create an embedding provider from configuration
pub fn create_provider(config: ProviderConfig) -> DomainResult<Box<dyn EmbeddingProvider>> {
    match config.provider {
        EmbeddingProviderType::OpenAI => {
            let api_key = config.api_key.or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| DomainError::Configuration(
                    "OpenAI API key not found. Set OPENAI_API_KEY environment variable or provide api_key".to_string()
                ))?;

            let mut openai_config = EmbeddingConfig::new(api_key);
            if let Some(model) = config.model {
                openai_config.model = model;
            }

            let client = OpenAIEmbeddingClient::new(openai_config)?;
            Ok(Box::new(client))
        }
        EmbeddingProviderType::Ollama => {
            let mut ollama_config = OllamaConfig::default();
            if let Some(model) = config.model {
                ollama_config = OllamaConfig::new(model);
            }
            if let Some(endpoint) = config.endpoint {
                ollama_config = ollama_config.with_endpoint(endpoint);
            }

            let client = OllamaEmbeddingClient::new(ollama_config)?;
            Ok(Box::new(client))
        }
        EmbeddingProviderType::Cohere => {
            let api_key = config.api_key.or_else(|| std::env::var("COHERE_API_KEY").ok())
                .ok_or_else(|| DomainError::Configuration(
                    "Cohere API key not found. Set COHERE_API_KEY environment variable or provide api_key".to_string()
                ))?;

            let mut cohere_config = CohereConfig::new(api_key);
            if let Some(model) = config.model {
                // Determine dimension based on model
                let dimension = match model.as_str() {
                    "embed-multilingual-v3.0" | "embed-english-v3.0" => 1024,
                    "embed-multilingual-light-v3.0" | "embed-english-light-v3.0" => 384,
                    _ => 1024,
                };
                cohere_config = cohere_config.with_model(model, dimension);
            }

            let client = CohereEmbeddingClient::new(cohere_config)?;
            Ok(Box::new(client))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_parsing() {
        assert_eq!(
            "openai".parse::<EmbeddingProviderType>().unwrap(),
            EmbeddingProviderType::OpenAI
        );
        assert_eq!(
            "ollama".parse::<EmbeddingProviderType>().unwrap(),
            EmbeddingProviderType::Ollama
        );
        assert_eq!(
            "cohere".parse::<EmbeddingProviderType>().unwrap(),
            EmbeddingProviderType::Cohere
        );
        assert!("unknown".parse::<EmbeddingProviderType>().is_err());
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::openai("test-key")
            .with_model("text-embedding-3-large");

        assert_eq!(config.provider, EmbeddingProviderType::OpenAI);
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, Some("text-embedding-3-large".to_string()));
    }
}
