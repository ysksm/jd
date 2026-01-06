//! Sync service

use std::sync::Arc;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbFieldRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository, DuckDbMetadataRepository,
    DuckDbSyncHistoryRepository, JiraApiClient, JiraConfig, SyncFieldsUseCase, SyncProjectUseCase,
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
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
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

    // Get projects to sync
    let projects_to_sync: Vec<_> = if let Some(ref project_key) = request.project_key {
        settings
            .projects
            .iter()
            .filter(|p| &p.key == project_key)
            .collect()
    } else {
        settings
            .projects
            .iter()
            .filter(|p| p.sync_enabled)
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

    // Step 3: Execute sync for each project (incremental if last_synced exists)
    let mut results = Vec::new();
    for project in &projects_to_sync {
        let start_time = std::time::Instant::now();
        let result = sync_use_case
            .execute(&project.key, &project.id, project.last_synced)
            .await;

        // Step 4: Expand issues for this project
        let _ = fields_use_case.expand_issues(Some(&project.id));

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(sync_result) => {
                results.push(SyncResult {
                    project_key: sync_result.project_key,
                    issue_count: sync_result.issues_synced as i32,
                    metadata_updated: true,
                    duration,
                    success: sync_result.success,
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
