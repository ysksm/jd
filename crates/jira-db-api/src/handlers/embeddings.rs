//! Embeddings handlers

use std::sync::Arc;

use axum::Json;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Generate embeddings for semantic search
pub async fn generate(
    _state: Arc<AppState>,
    _request: EmbeddingsGenerateRequest,
) -> Result<Json<EmbeddingsGenerateResponse>, ApiError> {
    // TODO: Implement embedding generation
    Err(ApiError::internal("Not implemented yet"))
}

/// Semantic search using embeddings
pub async fn search(
    _state: Arc<AppState>,
    _request: SemanticSearchRequest,
) -> Result<Json<SemanticSearchResponse>, ApiError> {
    // TODO: Implement semantic search
    Err(ApiError::internal("Not implemented yet"))
}
