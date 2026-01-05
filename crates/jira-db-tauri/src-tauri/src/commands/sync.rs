//! Sync command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbFieldRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository, DuckDbMetadataRepository,
    DuckDbSyncHistoryRepository, JiraApiClient, JiraConfig, RawDataRepository, SyncFieldsUseCase,
    SyncProjectUseCase,
};
use serde::{Deserialize, Serialize};

use crate::generated::*;
use crate::logging::Logger;
use crate::state::AppState;
use crate::{log_debug, log_info, log_warn};

// ============================================================
// Extended Response Types with Fields Expansion Stats
// ============================================================

/// Extended sync result with fields expansion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultExtended {
    #[serde(rename = "projectKey")]
    pub project_key: String,
    #[serde(rename = "issueCount")]
    pub issue_count: i32,
    #[serde(rename = "metadataUpdated")]
    pub metadata_updated: bool,
    pub duration: f64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    // Fields expansion stats
    #[serde(rename = "fieldsSynced")]
    pub fields_synced: i32,
    #[serde(rename = "columnsAdded")]
    pub columns_added: i32,
    #[serde(rename = "issuesExpanded")]
    pub issues_expanded: i32,
}

/// Extended sync response with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncExecuteResponseExtended {
    pub results: Vec<SyncResultExtended>,
    /// Total fields synced across all projects
    #[serde(rename = "totalFieldsSynced")]
    pub total_fields_synced: i32,
}

/// Execute sync for enabled projects with automatic fields expansion
#[tauri::command]
pub async fn sync_execute(
    state: State<'_, AppState>,
    request: SyncExecuteRequest,
) -> Result<SyncExecuteResponseExtended, String> {
    let log = Logger::new("sync");

    let settings = state.get_settings().ok_or("Not initialized")?;
    let db_factory = state
        .get_db_factory()
        .ok_or("Database factory not initialized")?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

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
        return Err("No projects to sync".to_string());
    }

    log_info!(
        log,
        "Starting sync for {} project(s)",
        projects_to_sync.len()
    );

    // Execute sync for each project with separate database
    let mut results = Vec::new();
    let mut total_fields_synced = 0i32;

    for project in &projects_to_sync {
        let start_time = std::time::Instant::now();
        log_info!(log, "[{}] Starting sync...", project.key);

        // Get database connection for this project
        let db = db_factory
            .get_connection(&project.key)
            .map_err(|e| format!("Failed to get database for {}: {}", project.key, e))?;

        // Get raw database connection for this project
        let raw_db = db_factory
            .get_raw_connection(&project.key)
            .map_err(|e| format!("Failed to get raw database for {}: {}", project.key, e))?;

        // Create repositories for sync
        let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
        let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db.clone()));
        let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db.clone()));
        let sync_history_repo = Arc::new(DuckDbSyncHistoryRepository::new(db.clone()));
        let snapshot_repo = Arc::new(DuckDbIssueSnapshotRepository::new(db.clone()));
        let raw_repo = Arc::new(RawDataRepository::new(raw_db));

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
        )
        .with_raw_repository(raw_repo);

        let fields_use_case =
            SyncFieldsUseCase::new(jira_client.clone(), field_repo, expanded_repo);

        // Step 1: Sync fields from JIRA
        log_info!(log, "[{}] Fetching JIRA fields...", project.key);
        let fields_synced = fields_use_case
            .sync_fields()
            .await
            .map_err(|e| e.to_string())? as i32;
        log_info!(log, "[{}] Synced {} fields", project.key, fields_synced);
        total_fields_synced = fields_synced;

        // Step 2: Add columns based on fields
        log_info!(log, "[{}] Adding database columns...", project.key);
        let added_columns = fields_use_case.add_columns().map_err(|e| e.to_string())?;
        let total_columns_added = added_columns.len() as i32;
        if total_columns_added > 0 {
            log_info!(
                log,
                "[{}] Added {} new columns: {:?}",
                project.key,
                total_columns_added,
                added_columns
            );
        }

        // Step 3: Execute sync
        log_info!(log, "[{}] Fetching issues from JIRA...", project.key);
        let result = sync_use_case.execute(&project.key, &project.id).await;

        // Step 4: Expand issues for this project
        log_info!(log, "[{}] Expanding issues...", project.key);
        let (issues_expanded, expand_error) = match fields_use_case.expand_issues(Some(&project.id))
        {
            Ok(count) => {
                log_info!(log, "[{}] Expanded {} issues", project.key, count);
                (count as i32, None)
            }
            Err(e) => {
                log_warn!(log, "[{}] Failed to expand issues: {}", project.key, e);
                (0, Some(e.to_string()))
            }
        };

        // Step 5: Create readable views
        log_info!(log, "[{}] Creating readable views...", project.key);
        if let Err(e) = fields_use_case.create_readable_view() {
            log_warn!(
                log,
                "[{}] Failed to create readable view: {}",
                project.key,
                e
            );
        }
        if let Err(e) = fields_use_case.create_snapshots_readable_view() {
            log_warn!(
                log,
                "[{}] Failed to create snapshot readable views: {}",
                project.key,
                e
            );
        }

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(sync_result) => {
                let error = sync_result.error_message.or(expand_error);
                log_info!(
                    log,
                    "[{}] Sync completed: {} issues in {:.1}s",
                    project.key,
                    sync_result.issues_synced,
                    duration
                );
                results.push(SyncResultExtended {
                    project_key: sync_result.project_key,
                    issue_count: sync_result.issues_synced as i32,
                    metadata_updated: true,
                    duration,
                    success: sync_result.success && error.is_none(),
                    error,
                    fields_synced,
                    columns_added: total_columns_added,
                    issues_expanded,
                });
            }
            Err(e) => {
                log_warn!(log, "[{}] Sync failed: {}", project.key, e);
                results.push(SyncResultExtended {
                    project_key: project.key.clone(),
                    issue_count: 0,
                    metadata_updated: false,
                    duration,
                    success: false,
                    error: Some(e.to_string()),
                    fields_synced: 0,
                    columns_added: 0,
                    issues_expanded: 0,
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

    log_info!(
        log,
        "Sync complete: {} projects, {} fields synced",
        results.len(),
        total_fields_synced
    );

    // Close database connections after sync to free resources
    for project in &projects_to_sync {
        if let Err(e) = state.close_db(&project.key) {
            log_warn!(log, "Failed to close database for {}: {}", project.key, e);
        }
    }
    log_debug!(
        log,
        "Closed database connections after sync, {} connections remaining",
        state.open_db_count()
    );

    Ok(SyncExecuteResponseExtended {
        results,
        total_fields_synced,
    })
}

/// Get sync status
#[tauri::command]
pub async fn sync_status(
    _state: State<'_, AppState>,
    _request: SyncStatusRequest,
) -> Result<SyncStatusResponse, String> {
    Ok(SyncStatusResponse {
        in_progress: false,
        progress: None,
    })
}
