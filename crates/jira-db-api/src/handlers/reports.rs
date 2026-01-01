//! Reports handlers

use std::sync::Arc;

use axum::Json;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Generate HTML report
pub async fn generate(
    _state: Arc<AppState>,
    _request: ReportGenerateRequest,
) -> Result<Json<ReportGenerateResponse>, ApiError> {
    // TODO: Implement report generation
    Err(ApiError::internal("Not implemented yet"))
}
