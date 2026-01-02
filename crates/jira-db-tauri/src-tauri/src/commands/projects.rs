//! Project command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{DuckDbProjectRepository, JiraApiClient, JiraConfig, SyncProjectListUseCase};

use crate::generated::*;
use crate::state::AppState;

/// List all projects
#[tauri::command]
pub async fn projects_list(
    state: State<'_, AppState>,
    _request: ProjectListRequest,
) -> Result<ProjectListResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    let projects = settings
        .projects
        .into_iter()
        .map(|p| Project {
            id: p.id,
            key: p.key,
            name: p.name,
            description: None,
            enabled: p.sync_enabled,
            last_synced_at: p.last_synced,
        })
        .collect();

    Ok(ProjectListResponse { projects })
}

/// Initialize projects from JIRA
#[tauri::command]
pub async fn projects_initialize(
    state: State<'_, AppState>,
    _request: ProjectInitRequest,
) -> Result<ProjectInitResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create project repository
    let project_repo = Arc::new(DuckDbProjectRepository::new(db));

    // Execute use case
    let use_case = SyncProjectListUseCase::new(project_repo, jira_client);
    let fetched_projects = use_case.execute().await.map_err(|e| e.to_string())?;

    let new_count = fetched_projects.len() as i32;

    // Update settings with new projects
    let updated_settings = state
        .update_settings(|s| {
            for project in &fetched_projects {
                let exists = s.projects.iter().any(|p| p.key == project.key);
                if !exists {
                    s.projects.push(jira_db_core::ProjectConfig {
                        id: project.id.clone(),
                        key: project.key.clone(),
                        name: project.name.clone(),
                        sync_enabled: false,
                        last_synced: None,
                    });
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let projects = updated_settings
        .projects
        .into_iter()
        .map(|p| Project {
            id: p.id,
            key: p.key,
            name: p.name,
            description: None,
            enabled: p.sync_enabled,
            last_synced_at: p.last_synced,
        })
        .collect();

    Ok(ProjectInitResponse {
        projects,
        new_count,
    })
}

/// Enable project sync
#[tauri::command]
pub async fn projects_enable(
    state: State<'_, AppState>,
    request: ProjectEnableRequest,
) -> Result<ProjectEnableResponse, String> {
    let updated_settings = state
        .update_settings(|s| {
            if let Some(project) = s.find_project_mut(&request.key) {
                project.sync_enabled = true;
            }
        })
        .map_err(|e| e.to_string())?;

    let project = updated_settings
        .projects
        .into_iter()
        .find(|p| p.key == request.key)
        .map(|p| Project {
            id: p.id,
            key: p.key,
            name: p.name,
            description: None,
            enabled: p.sync_enabled,
            last_synced_at: p.last_synced,
        })
        .ok_or("Project not found")?;

    Ok(ProjectEnableResponse { project })
}

/// Disable project sync
#[tauri::command]
pub async fn projects_disable(
    state: State<'_, AppState>,
    request: ProjectDisableRequest,
) -> Result<ProjectDisableResponse, String> {
    let updated_settings = state
        .update_settings(|s| {
            if let Some(project) = s.find_project_mut(&request.key) {
                project.sync_enabled = false;
            }
        })
        .map_err(|e| e.to_string())?;

    let project = updated_settings
        .projects
        .into_iter()
        .find(|p| p.key == request.key)
        .map(|p| Project {
            id: p.id,
            key: p.key,
            name: p.name,
            description: None,
            enabled: p.sync_enabled,
            last_synced_at: p.last_synced,
        })
        .ok_or("Project not found")?;

    Ok(ProjectDisableResponse { project })
}
