//! Generated handlers from TypeSpec
//!
//! Route definitions and handler stubs for the API.

use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};

use super::types::*;
use crate::error::ApiError;
use crate::handlers;
use crate::state::AppState;

/// Create router with all API endpoints
pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        // Config
        .route("/config.get", post(config_get))
        .route("/config.update", post(config_update))
        .route("/config.init", post(config_init))
        // Projects
        .route("/projects.list", post(projects_list))
        .route("/projects.init", post(projects_init))
        .route("/projects.enable", post(projects_enable))
        .route("/projects.disable", post(projects_disable))
        // Sync
        .route("/sync.execute", post(sync_execute))
        .route("/sync.status", post(sync_status))
        // Issues
        .route("/issues.search", post(issues_search))
        .route("/issues.get", post(issues_get))
        .route("/issues.history", post(issues_history))
        // Metadata
        .route("/metadata.get", post(metadata_get))
        // Embeddings
        .route("/embeddings.generate", post(embeddings_generate))
        .route("/embeddings.search", post(embeddings_search))
        // Reports
        .route("/reports.generate", post(reports_generate))
}

// ============================================================
// Config Handlers
// ============================================================

/// Get current configuration
async fn config_get(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfigGetRequest>,
) -> Result<Json<ConfigGetResponse>, ApiError> {
    handlers::config::get(state, request).await
}

/// Update configuration
async fn config_update(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigUpdateResponse>, ApiError> {
    handlers::config::update(state, request).await
}

/// Initialize configuration
async fn config_init(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfigInitRequest>,
) -> Result<Json<ConfigInitResponse>, ApiError> {
    handlers::config::init(state, request).await
}

// ============================================================
// Projects Handlers
// ============================================================

/// List all projects
async fn projects_list(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProjectListRequest>,
) -> Result<Json<ProjectListResponse>, ApiError> {
    handlers::projects::list(state, request).await
}

/// Initialize projects from JIRA
async fn projects_init(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProjectInitRequest>,
) -> Result<Json<ProjectInitResponse>, ApiError> {
    handlers::projects::init(state, request).await
}

/// Enable project sync
async fn projects_enable(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProjectEnableRequest>,
) -> Result<Json<ProjectEnableResponse>, ApiError> {
    handlers::projects::enable(state, request).await
}

/// Disable project sync
async fn projects_disable(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProjectDisableRequest>,
) -> Result<Json<ProjectDisableResponse>, ApiError> {
    handlers::projects::disable(state, request).await
}

// ============================================================
// Sync Handlers
// ============================================================

/// Execute sync for enabled projects
async fn sync_execute(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SyncExecuteRequest>,
) -> Result<Json<SyncExecuteResponse>, ApiError> {
    handlers::sync::execute(state, request).await
}

/// Get sync status
async fn sync_status(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SyncStatusRequest>,
) -> Result<Json<SyncStatusResponse>, ApiError> {
    handlers::sync::status(state, request).await
}

// ============================================================
// Issues Handlers
// ============================================================

/// Search issues with filters
async fn issues_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IssueSearchRequest>,
) -> Result<Json<IssueSearchResponse>, ApiError> {
    handlers::issues::search(state, request).await
}

/// Get issue by key
async fn issues_get(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IssueGetRequest>,
) -> Result<Json<IssueGetResponse>, ApiError> {
    handlers::issues::get(state, request).await
}

/// Get issue change history
async fn issues_history(
    State(state): State<Arc<AppState>>,
    Json(request): Json<IssueHistoryRequest>,
) -> Result<Json<IssueHistoryResponse>, ApiError> {
    handlers::issues::history(state, request).await
}

// ============================================================
// Metadata Handlers
// ============================================================

/// Get project metadata
async fn metadata_get(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MetadataGetRequest>,
) -> Result<Json<MetadataGetResponse>, ApiError> {
    handlers::metadata::get(state, request).await
}

// ============================================================
// Embeddings Handlers
// ============================================================

/// Generate embeddings for semantic search
async fn embeddings_generate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<EmbeddingsGenerateRequest>,
) -> Result<Json<EmbeddingsGenerateResponse>, ApiError> {
    handlers::embeddings::generate(state, request).await
}

/// Semantic search using embeddings
async fn embeddings_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SemanticSearchRequest>,
) -> Result<Json<SemanticSearchResponse>, ApiError> {
    handlers::embeddings::search(state, request).await
}

// ============================================================
// Reports Handlers
// ============================================================

/// Generate HTML report
async fn reports_generate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReportGenerateRequest>,
) -> Result<Json<ReportGenerateResponse>, ApiError> {
    handlers::reports::generate(state, request).await
}
