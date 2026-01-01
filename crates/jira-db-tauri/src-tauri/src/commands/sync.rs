//! Sync command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbSyncHistoryRepository, JiraApiClient, SyncProjectUseCase,
};

use crate::generated::*;
use crate::state::AppState;

/// Execute sync for enabled projects
#[tauri::command]
pub async fn sync_execute(
    state: State<'_, AppState>,
    request: SyncExecuteRequest,
) -> Result<SyncExecuteResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create JIRA client
    let jira_client = Arc::new(
        JiraApiClient::new(
            &settings.jira.endpoint,
            &settings.jira.username,
            &settings.jira.api_key,
        )
        .map_err(|e| e.to_string())?,
    );

    // Create repositories
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db.clone()));
    let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db.clone()));
    let sync_history_repo = Arc::new(DuckDbSyncHistoryRepository::new(db));

    // Create use case
    let use_case = SyncProjectUseCase::new(
        issue_repo,
        change_history_repo,
        metadata_repo,
        sync_history_repo,
        jira_client,
    );

    // Get projects to sync
    let projects_to_sync: Vec<_> = if let Some(ref project_key) = request.project_key {
        // Sync specific project
        settings
            .projects
            .iter()
            .filter(|p| &p.key == project_key)
            .collect()
    } else {
        // Sync all enabled projects
        settings
            .projects
            .iter()
            .filter(|p| p.sync_enabled)
            .collect()
    };

    if projects_to_sync.is_empty() {
        return Err("No projects to sync".to_string());
    }

    // Execute sync for each project
    let mut results = Vec::new();
    for project in projects_to_sync {
        let start_time = std::time::Instant::now();
        let result = use_case.execute(&project.key, &project.id).await;
        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(sync_result) => {
                let has_error = sync_result.error.is_some();
                results.push(SyncResult {
                    project_key: sync_result.project_key,
                    issue_count: sync_result.issues_synced as i32,
                    metadata_updated: sync_result.metadata_synced,
                    duration,
                    success: !has_error,
                    error: sync_result.error,
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
        .map_err(|e| e.to_string())?;

    Ok(SyncExecuteResponse { results })
}

/// Get sync status
#[tauri::command]
pub async fn sync_status(
    _state: State<'_, AppState>,
    _request: SyncStatusRequest,
) -> Result<SyncStatusResponse, String> {
    // TODO: Implement proper progress tracking with shared state
    Ok(SyncStatusResponse {
        in_progress: false,
        progress: None,
    })
}
