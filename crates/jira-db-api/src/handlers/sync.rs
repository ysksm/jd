//! Sync handlers

use std::sync::Arc;

use axum::Json;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Execute sync for enabled projects
pub async fn execute(
    _state: Arc<AppState>,
    _request: SyncExecuteRequest,
) -> Result<Json<SyncExecuteResponse>, ApiError> {
    // TODO: Implement sync execution
    Err(ApiError::internal("Not implemented yet"))
}

/// Get sync status
pub async fn status(
    _state: Arc<AppState>,
    _request: SyncStatusRequest,
) -> Result<Json<SyncStatusResponse>, ApiError> {
    // For now, return not in progress
    Ok(Json(SyncStatusResponse {
        in_progress: false,
        progress: None,
    }))
}
