//! Embeddings command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Generate embeddings for semantic search
#[tauri::command]
pub async fn embeddings_generate(
    _state: State<'_, AppState>,
    _request: EmbeddingsGenerateRequest,
) -> Result<EmbeddingsGenerateResponse, String> {
    Err("Not implemented".to_string())
}

/// Semantic search using embeddings
#[tauri::command]
pub async fn embeddings_search(
    _state: State<'_, AppState>,
    _request: SemanticSearchRequest,
) -> Result<SemanticSearchResponse, String> {
    Err("Not implemented".to_string())
}
