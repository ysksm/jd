//! Sync service

use std::sync::Arc;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbFieldRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository, DuckDbMetadataRepository,
    DuckDbSyncHistoryRepository, JiraApiClient, Settings, SyncCheckpoint, SyncFieldsUseCase,
    SyncProjectUseCase,
};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

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

    // Create JIRA config and client from active endpoint
    let jira_config = settings
        .get_jira_config()
        .ok_or_else(|| ServiceError::Config("No JIRA endpoint configured".to_string()))?;
    let jira_client = Arc::new(
        JiraApiClient::new(&jira_config).map_err(|e| ServiceError::JiraApi(e.to_string()))?,
    );

    // Create repositories for sync
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db.clone()));
    let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db.clone()));
    let sync_history_repo = Arc::new(DuckDbSyncHistoryRepository::new(db.clone()));
    let snapshot_repo = Arc::new(DuckDbIssueSnapshotRepository::new(db.clone()));

    // Create repositories for fields expansion
    let field_repo = Arc::new(DuckDbFieldRepository::new(db.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(db));

    // Create use cases
    let sync_use_case = SyncProjectUseCase::new(
        issue_repo,
        change_history_repo,
        metadata_repo,
        sync_history_repo,
        snapshot_repo,
        jira_client.clone(),
    );

    let fields_use_case = SyncFieldsUseCase::new(jira_client, field_repo, expanded_repo);

    // Get projects to sync with their checkpoint information
    let projects_to_sync: Vec<(String, String, Option<SyncCheckpoint>)> =
        if let Some(ref project_key) = request.project_key {
            settings
                .projects
                .iter()
                .filter(|p| &p.key == project_key)
                .map(|p| (p.key.clone(), p.id.clone(), p.sync_checkpoint.clone()))
                .collect()
        } else {
            settings
                .projects
                .iter()
                .filter(|p| p.sync_enabled)
                .map(|p| (p.key.clone(), p.id.clone(), p.sync_checkpoint.clone()))
                .collect()
        };

    if projects_to_sync.is_empty() {
        return Err(ServiceError::InvalidRequest(
            "No projects to sync".to_string(),
        ));
    }

    // Step 1: Sync fields from JIRA
    tracing::info!("Syncing fields from JIRA...");
    let _fields_synced = fields_use_case
        .sync_fields()
        .await
        .map_err(|e| ServiceError::JiraApi(e.to_string()))?;

    // Step 2: Add columns based on fields
    tracing::info!("Adding columns to issues_expanded table...");
    let _ = fields_use_case
        .add_columns()
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    // Step 3: Execute resumable sync for each project with checkpoint support
    let mut results = Vec::new();
    for (key, id, checkpoint) in &projects_to_sync {
        let start_time = std::time::Instant::now();

        // Show resuming message if we have a checkpoint
        if let Some(cp) = checkpoint {
            tracing::info!(
                "[{}] Resuming sync from checkpoint ({}/{} issues processed)",
                key,
                cp.items_processed,
                cp.total_items
            );
        }

        // Clone values for the checkpoint callback
        let settings_path_clone = settings_path.clone();
        let key_clone = key.clone();

        // Use resumable sync with checkpoint saving callback
        let result = sync_use_case
            .execute_resumable(key, id, checkpoint.clone(), move |new_checkpoint| {
                // Save checkpoint to settings after each batch
                if let Ok(mut s) = Settings::load(&settings_path_clone) {
                    if let Some(p) = s.find_project_mut(&key_clone) {
                        p.sync_checkpoint = Some(new_checkpoint.clone());
                    }
                    let _ = s.save(&settings_path_clone);
                }
            })
            .await;

        // Step 4: Expand issues for this project
        let _ = fields_use_case.expand_issues(Some(id));

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(resumable_result) => {
                let sync_result = resumable_result.sync_result;
                let success = sync_result.success;

                if success {
                    // Clear checkpoint on success
                    if let Ok(mut s) = Settings::load(&settings_path) {
                        if let Some(p) = s.find_project_mut(key) {
                            p.sync_checkpoint = None;
                        }
                        let _ = s.save(&settings_path);
                    }
                } else {
                    // Save checkpoint for resume on failure
                    if let Some(checkpoint) = resumable_result.checkpoint {
                        if let Ok(mut s) = Settings::load(&settings_path) {
                            if let Some(p) = s.find_project_mut(key) {
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
                    project_key: key.clone(),
                    issue_count: 0,
                    metadata_updated: false,
                    duration,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Step 5: Create readable views
    let _ = fields_use_case.create_readable_view();
    let _ = fields_use_case.create_snapshots_readable_view();

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
