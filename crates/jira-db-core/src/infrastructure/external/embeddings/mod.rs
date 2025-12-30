//! Embedding providers for vector search
//!
//! This module provides integration with embedding providers like OpenAI
//! for generating vector embeddings from text.

mod openai;

pub use openai::{OpenAIEmbeddingClient, EmbeddingConfig, EmbeddingResult};

use async_trait::async_trait;
use crate::domain::error::DomainResult;

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
