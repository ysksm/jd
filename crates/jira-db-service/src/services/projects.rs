//! Projects service

use std::sync::Arc;

use jira_db_core::{DuckDbProjectRepository, JiraApiClient, SyncProjectListUseCase};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// List all projects
pub fn list(state: &AppState, _request: ProjectListRequest) -> ServiceResult<ProjectListResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;

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
pub async fn initialize(
    state: &AppState,
    _request: ProjectInitRequest,
) -> ServiceResult<ProjectInitResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Create JIRA config and client from active endpoint
    let jira_config = settings
        .get_jira_config()
        .ok_or_else(|| ServiceError::Config("No JIRA endpoint configured".to_string()))?;
    let jira_client = Arc::new(
        JiraApiClient::new(&jira_config).map_err(|e| ServiceError::JiraApi(e.to_string()))?,
    );

    // Create project repository
    let project_repo = Arc::new(DuckDbProjectRepository::new(db));

    // Execute use case
    let use_case = SyncProjectListUseCase::new(project_repo, jira_client);
    let fetched_projects = use_case
        .execute()
        .await
        .map_err(|e| ServiceError::JiraApi(e.to_string()))?;

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
                        endpoint: None, // Uses active endpoint by default
                        sync_checkpoint: None,
                        snapshot_checkpoint: None,
                    });
                }
            }
        })
        .map_err(|e| ServiceError::Config(e.to_string()))?;

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
pub fn enable(
    state: &AppState,
    request: ProjectEnableRequest,
) -> ServiceResult<ProjectEnableResponse> {
    let updated_settings = state
        .update_settings(|s| {
            if let Some(project) = s.find_project_mut(&request.key) {
                project.sync_enabled = true;
            }
        })
        .map_err(|e| ServiceError::Config(e.to_string()))?;

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
        .ok_or_else(|| ServiceError::NotFound("Project not found".to_string()))?;

    Ok(ProjectEnableResponse { project })
}

/// Disable project sync
pub fn disable(
    state: &AppState,
    request: ProjectDisableRequest,
) -> ServiceResult<ProjectDisableResponse> {
    let updated_settings = state
        .update_settings(|s| {
            if let Some(project) = s.find_project_mut(&request.key) {
                project.sync_enabled = false;
            }
        })
        .map_err(|e| ServiceError::Config(e.to_string()))?;

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
        .ok_or_else(|| ServiceError::NotFound("Project not found".to_string()))?;

    Ok(ProjectDisableResponse { project })
}
