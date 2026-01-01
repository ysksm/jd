//! Project command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// List all projects
#[tauri::command]
pub async fn projects_list(
    _state: State<'_, AppState>,
    _request: ProjectListRequest,
) -> Result<ProjectListResponse, String> {
    Err("Not implemented".to_string())
}

/// Initialize projects from JIRA
#[tauri::command]
pub async fn projects_init(
    _state: State<'_, AppState>,
    _request: ProjectInitRequest,
) -> Result<ProjectInitResponse, String> {
    Err("Not implemented".to_string())
}

/// Enable project sync
#[tauri::command]
pub async fn projects_enable(
    _state: State<'_, AppState>,
    _request: ProjectEnableRequest,
) -> Result<ProjectEnableResponse, String> {
    Err("Not implemented".to_string())
}

/// Disable project sync
#[tauri::command]
pub async fn projects_disable(
    _state: State<'_, AppState>,
    _request: ProjectDisableRequest,
) -> Result<ProjectDisableResponse, String> {
    Err("Not implemented".to_string())
}
