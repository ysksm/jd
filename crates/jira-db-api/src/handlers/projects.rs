//! Projects handlers

use std::sync::Arc;

use axum::Json;
use jira_db_core::DuckDbProjectRepository;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// List all projects
pub async fn list(
    state: Arc<AppState>,
    _request: ProjectListRequest,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let repo = DuckDbProjectRepository::new(state.db.clone());
    let projects = repo.find_all()?;

    let api_projects: Vec<Project> = projects
        .into_iter()
        .map(|p| Project {
            id: p.id,
            key: p.key,
            name: p.name,
            description: p.description,
            enabled: p.enabled,
            last_synced_at: p.last_synced_at,
        })
        .collect();

    Ok(Json(ProjectListResponse {
        projects: api_projects,
    }))
}

/// Initialize projects from JIRA
pub async fn init(
    _state: Arc<AppState>,
    _request: ProjectInitRequest,
) -> Result<Json<ProjectInitResponse>, ApiError> {
    // TODO: Implement project initialization from JIRA
    Err(ApiError::internal("Not implemented yet"))
}

/// Enable project sync
pub async fn enable(
    _state: Arc<AppState>,
    _request: ProjectEnableRequest,
) -> Result<Json<ProjectEnableResponse>, ApiError> {
    // TODO: Implement project enable
    Err(ApiError::internal("Not implemented yet"))
}

/// Disable project sync
pub async fn disable(
    _state: Arc<AppState>,
    _request: ProjectDisableRequest,
) -> Result<Json<ProjectDisableResponse>, ApiError> {
    // TODO: Implement project disable
    Err(ApiError::internal("Not implemented yet"))
}
