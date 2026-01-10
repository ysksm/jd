//! Sync service

use std::collections::HashMap;
use std::sync::Arc;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbFieldRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository, DuckDbMetadataRepository,
    DuckDbSyncHistoryRepository, JiraApiClient, JiraConfig, Settings, SyncCheckpoint,
    SyncFieldsUseCase, SyncProjectUseCase,
};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// Project info with endpoint association
struct ProjectSyncInfo {
    key: String,
    id: String,
    checkpoint: Option<SyncCheckpoint>,
    endpoint_name: Option<String>,
}

/// Execute sync for enabled projects
pub async fn execute(
    state: &AppState,
    request: SyncExecuteRequest,
) -> ServiceResult<SyncExecuteResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let settings_path = state
        .get_settings_path()
        .ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Create repositories for sync (shared across all projects)
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db.clone()));
    let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db.clone()));
    let sync_history_repo = Arc::new(DuckDbSyncHistoryRepository::new(db.clone()));
    let snapshot_repo = Arc::new(DuckDbIssueSnapshotRepository::new(db.clone()));

    // Create repositories for fields expansion
    let field_repo = Arc::new(DuckDbFieldRepository::new(db.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(db));

    // Get projects to sync with their endpoint information
    let projects_to_sync: Vec<ProjectSyncInfo> = if let Some(ref project_key) = request.project_key
    {
        settings
            .projects
            .iter()
            .filter(|p| &p.key == project_key)
            .map(|p| ProjectSyncInfo {
                key: p.key.clone(),
                id: p.id.clone(),
                checkpoint: p.sync_checkpoint.clone(),
                endpoint_name: p.endpoint.clone(),
            })
            .collect()
    } else {
        settings
            .projects
            .iter()
            .filter(|p| p.sync_enabled)
            .map(|p| ProjectSyncInfo {
                key: p.key.clone(),
                id: p.id.clone(),
                checkpoint: p.sync_checkpoint.clone(),
                endpoint_name: p.endpoint.clone(),
            })
            .collect()
    };

    if projects_to_sync.is_empty() {
        return Err(ServiceError::InvalidRequest(
            "No projects to sync".to_string(),
        ));
    }

    // Group projects by endpoint
    let mut projects_by_endpoint: HashMap<Option<String>, Vec<&ProjectSyncInfo>> = HashMap::new();
    for project in &projects_to_sync {
        projects_by_endpoint
            .entry(project.endpoint_name.clone())
            .or_default()
            .push(project);
    }

    // Helper function to get JIRA config for a project
    fn get_jira_config_for_project(
        settings: &jira_db_core::Settings,
        endpoint_name: &Option<String>,
    ) -> Option<JiraConfig> {
        if let Some(name) = endpoint_name {
            // Use project's specific endpoint
            settings.get_endpoint(name).map(|e| e.to_jira_config())
        } else {
            // Fall back to active endpoint
            settings.get_jira_config()
        }
    }

    let mut results = Vec::new();
    let mut fields_synced_for_endpoint: std::collections::HashSet<Option<String>> =
        std::collections::HashSet::new();

    // Process each endpoint group
    for (endpoint_name, projects) in &projects_by_endpoint {
        // Get JIRA config for this endpoint
        let jira_config = match get_jira_config_for_project(&settings, endpoint_name) {
            Some(config) => config,
            None => {
                let endpoint_display = endpoint_name
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("default");
                tracing::error!("No JIRA endpoint found for: {}", endpoint_display);
                for project in projects {
                    results.push(SyncResult {
                        project_key: project.key.clone(),
                        issue_count: 0,
                        metadata_updated: false,
                        duration: 0.0,
                        success: false,
                        error: Some(format!("JIRA endpoint '{}' not found", endpoint_display)),
                    });
                }
                continue;
            }
        };

        // Create JIRA client for this endpoint
        let jira_client = match JiraApiClient::new(&jira_config) {
            Ok(client) => Arc::new(client),
            Err(e) => {
                let endpoint_display = endpoint_name
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("default");
                tracing::error!(
                    "Failed to create JIRA client for endpoint '{}': {}",
                    endpoint_display,
                    e
                );
                for project in projects {
                    results.push(SyncResult {
                        project_key: project.key.clone(),
                        issue_count: 0,
                        metadata_updated: false,
                        duration: 0.0,
                        success: false,
                        error: Some(format!("Failed to create JIRA client: {}", e)),
                    });
                }
                continue;
            }
        };

        let endpoint_display = endpoint_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("default");
        tracing::info!(
            "Syncing {} project(s) from endpoint '{}'",
            projects.len(),
            endpoint_display
        );

        // Create use cases for this endpoint
        let sync_use_case = SyncProjectUseCase::new(
            issue_repo.clone(),
            change_history_repo.clone(),
            metadata_repo.clone(),
            sync_history_repo.clone(),
            snapshot_repo.clone(),
            jira_client.clone(),
        );

        let fields_use_case = SyncFieldsUseCase::new(
            jira_client.clone(),
            field_repo.clone(),
            expanded_repo.clone(),
        );

        // Step 1: Sync fields from JIRA (once per endpoint)
        if !fields_synced_for_endpoint.contains(endpoint_name) {
            tracing::info!(
                "Syncing fields from JIRA endpoint '{}'...",
                endpoint_display
            );
            if let Err(e) = fields_use_case.sync_fields().await {
                tracing::warn!(
                    "Failed to sync fields from endpoint '{}': {}",
                    endpoint_display,
                    e
                );
            }
            fields_synced_for_endpoint.insert(endpoint_name.clone());

            // Step 2: Add columns based on fields
            tracing::info!("Adding columns to issues_expanded table...");
            let _ = fields_use_case.add_columns();
        }

        // Step 3: Execute resumable sync for each project with checkpoint support
        for project in projects {
            let start_time = std::time::Instant::now();

            // Show resuming message if we have a checkpoint
            if let Some(cp) = &project.checkpoint {
                tracing::info!(
                    "[{}] Resuming sync from checkpoint ({}/{} issues processed)",
                    project.key,
                    cp.items_processed,
                    cp.total_items
                );
            }

            // Clone values for the checkpoint callback
            let settings_path_clone = settings_path.clone();
            let key_clone = project.key.clone();

            // Use resumable sync with checkpoint saving callback
            let result = sync_use_case
                .execute_resumable(
                    &project.key,
                    &project.id,
                    project.checkpoint.clone(),
                    move |new_checkpoint| {
                        // Save checkpoint to settings after each batch
                        if let Ok(mut s) = Settings::load(&settings_path_clone) {
                            if let Some(p) = s.find_project_mut(&key_clone) {
                                p.sync_checkpoint = Some(new_checkpoint.clone());
                            }
                            let _ = s.save(&settings_path_clone);
                        }
                    },
                )
                .await;

            // Step 4: Expand issues for this project
            let _ = fields_use_case.expand_issues(Some(&project.id));

            let duration = start_time.elapsed().as_secs_f64();

            match result {
                Ok(resumable_result) => {
                    let sync_result = resumable_result.sync_result;
                    let success = sync_result.success;

                    if success {
                        // Clear checkpoint on success
                        if let Ok(mut s) = Settings::load(&settings_path) {
                            if let Some(p) = s.find_project_mut(&project.key) {
                                p.sync_checkpoint = None;
                            }
                            let _ = s.save(&settings_path);
                        }
                    } else {
                        // Save checkpoint for resume on failure
                        if let Some(checkpoint) = resumable_result.checkpoint {
                            if let Ok(mut s) = Settings::load(&settings_path) {
                                if let Some(p) = s.find_project_mut(&project.key) {
                                    p.sync_checkpoint = Some(checkpoint);
                                }
                                let _ = s.save(&settings_path);
                            }
                        }
                    }

                    results.push(SyncResult {
                        project_key: sync_result.project_key,
                        issue_count: sync_result.issues_synced as i32,
                        metadata_updated: true,
                        duration,
                        success,
                        error: sync_result.error_message,
                    });
                }
                Err(e) => {
                    results.push(SyncResult {
                        project_key: project.key.clone(),
                        issue_count: 0,
                        metadata_updated: false,
                        duration,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        // Step 5: Create readable views (once per endpoint)
        let _ = fields_use_case.create_readable_view();
        let _ = fields_use_case.create_snapshots_readable_view();
    }

    // Update last_synced for successful projects
    state
        .update_settings(|s| {
            for result in &results {
                if result.success {
                    if let Some(project) = s.find_project_mut(&result.project_key) {
                        project.last_synced = Some(jira_db_core::chrono::Utc::now());
                    }
                }
            }
        })
        .map_err(|e| ServiceError::Config(e.to_string()))?;

    Ok(SyncExecuteResponse { results })
}

/// Get sync status
pub fn status(_state: &AppState, _request: SyncStatusRequest) -> ServiceResult<SyncStatusResponse> {
    Ok(SyncStatusResponse {
        in_progress: false,
        progress: None,
    })
}
