//! Projects service

use std::sync::Arc;

use jira_db_core::{DuckDbProjectRepository, JiraApiClient, JiraEndpoint, SyncProjectListUseCase};

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
    request: ProjectInitRequest,
) -> ServiceResult<ProjectInitResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Determine which endpoints to fetch from
    let endpoints_to_fetch: Vec<&JiraEndpoint> = if request.all_endpoints {
        // Fetch from all endpoints
        settings.jira_endpoints.iter().collect()
    } else if let Some(ref endpoint_name) = request.endpoint_name {
        // Fetch from specific endpoint
        settings
            .get_endpoint(endpoint_name)
            .map(|e| vec![e])
            .unwrap_or_default()
    } else {
        // Fetch from active endpoint only
        settings
            .get_active_endpoint()
            .map(|e| vec![e])
            .unwrap_or_default()
    };

    if endpoints_to_fetch.is_empty() {
        return Err(ServiceError::Config(
            "No JIRA endpoint configured".to_string(),
        ));
    }

    let mut all_fetched_projects = Vec::new();
    let mut endpoint_results = Vec::new();
    let mut total_new_count = 0;

    // Fetch projects from each endpoint
    for endpoint in &endpoints_to_fetch {
        let jira_config = endpoint.to_jira_config();
        let endpoint_name = endpoint.name.clone();

        match JiraApiClient::new(&jira_config) {
            Ok(client) => {
                let jira_client = Arc::new(client);
                let project_repo = Arc::new(DuckDbProjectRepository::new(db.clone()));
                let use_case = SyncProjectListUseCase::new(project_repo, jira_client);

                match use_case.execute().await {
                    Ok(fetched_projects) => {
                        let count = fetched_projects.len() as i32;
                        tracing::info!(
                            "Fetched {} projects from endpoint '{}'",
                            count,
                            endpoint_name
                        );

                        // Store projects with their endpoint
                        for project in fetched_projects {
                            all_fetched_projects.push((project, endpoint_name.clone()));
                        }

                        endpoint_results.push(EndpointFetchResult {
                            endpoint_name: endpoint_name.clone(),
                            project_count: count,
                            success: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to fetch projects from endpoint '{}': {}",
                            endpoint_name,
                            e
                        );
                        endpoint_results.push(EndpointFetchResult {
                            endpoint_name: endpoint_name.clone(),
                            project_count: 0,
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create JIRA client for endpoint '{}': {}",
                    endpoint_name,
                    e
                );
                endpoint_results.push(EndpointFetchResult {
                    endpoint_name: endpoint_name.clone(),
                    project_count: 0,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Update settings with new projects
    let updated_settings = state
        .update_settings(|s| {
            for (project, endpoint_name) in &all_fetched_projects {
                let exists = s.projects.iter().any(|p| p.key == project.key);
                if !exists {
                    total_new_count += 1;
                    s.projects.push(jira_db_core::ProjectConfig {
                        id: project.id.clone(),
                        key: project.key.clone(),
                        name: project.name.clone(),
                        sync_enabled: false,
                        last_synced: None,
                        endpoint: Some(endpoint_name.clone()),
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
        new_count: total_new_count,
        endpoint_results: if endpoints_to_fetch.len() > 1 {
            Some(endpoint_results)
        } else {
            None
        },
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
