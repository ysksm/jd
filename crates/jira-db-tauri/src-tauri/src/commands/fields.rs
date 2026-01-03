//! Fields command handlers
//!
//! Commands for managing JIRA field metadata and expanding issues from raw_data.

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    DuckDbFieldRepository, DuckDbIssuesExpandedRepository, JiraApiClient, JiraConfig,
    SyncFieldsUseCase,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// ============================================================
// Request/Response Types for Fields Commands
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsSyncRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsSyncResponse {
    pub fields_synced: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsExpandRequest {
    #[serde(rename = "projectKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsExpandResponse {
    pub columns_added: i32,
    pub issues_expanded: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsFullRequest {
    #[serde(rename = "projectKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsFullResponse {
    pub fields_synced: i32,
    pub columns_added: i32,
    pub issues_expanded: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsListRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraFieldInfo {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub navigable: bool,
    #[serde(rename = "schemaType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldsListResponse {
    pub fields: Vec<JiraFieldInfo>,
    pub total: i32,
}

// ============================================================
// Command Handlers
// ============================================================

/// Sync fields from JIRA API
#[tauri::command]
pub async fn fields_sync(
    state: State<'_, AppState>,
    _request: FieldsSyncRequest,
) -> Result<FieldsSyncResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create repositories
    let field_repo = Arc::new(DuckDbFieldRepository::new(db.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(db));

    // Create use case
    let use_case = SyncFieldsUseCase::new(jira_client, field_repo, expanded_repo);

    // Execute sync
    let fields_synced = use_case.sync_fields().await.map_err(|e| e.to_string())?;

    tracing::info!("Synced {} fields from JIRA", fields_synced);

    Ok(FieldsSyncResponse {
        fields_synced: fields_synced as i32,
    })
}

/// Expand issues from raw_data into issues_expanded table
#[tauri::command]
pub async fn fields_expand(
    state: State<'_, AppState>,
    request: FieldsExpandRequest,
) -> Result<FieldsExpandResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create repositories
    let field_repo = Arc::new(DuckDbFieldRepository::new(db.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(db));

    // Create use case
    let use_case = SyncFieldsUseCase::new(jira_client, field_repo, expanded_repo);

    // Add columns
    let added_columns = use_case.add_columns().map_err(|e| e.to_string())?;
    let columns_added = added_columns.len();

    // Expand issues
    let issues_expanded = use_case
        .expand_issues(request.project_key.as_deref())
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Added {} columns, expanded {} issues",
        columns_added,
        issues_expanded
    );

    Ok(FieldsExpandResponse {
        columns_added: columns_added as i32,
        issues_expanded: issues_expanded as i32,
    })
}

/// Full sync: fetch fields, add columns, and expand issues
#[tauri::command]
pub async fn fields_full(
    state: State<'_, AppState>,
    request: FieldsFullRequest,
) -> Result<FieldsFullResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create JIRA config and client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create repositories
    let field_repo = Arc::new(DuckDbFieldRepository::new(db.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(db));

    // Create use case
    let use_case = SyncFieldsUseCase::new(jira_client, field_repo, expanded_repo);

    // Execute full sync
    let result = use_case
        .execute(request.project_key.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Fields full sync: {} fields synced, {} columns added, {} issues expanded",
        result.fields_synced,
        result.columns_added,
        result.issues_expanded
    );

    Ok(FieldsFullResponse {
        fields_synced: result.fields_synced as i32,
        columns_added: result.columns_added as i32,
        issues_expanded: result.issues_expanded as i32,
    })
}

/// List all stored fields
#[tauri::command]
pub async fn fields_list(
    state: State<'_, AppState>,
    _request: FieldsListRequest,
) -> Result<FieldsListResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    // Create repository
    let field_repo = DuckDbFieldRepository::new(db);

    // Get all fields
    let fields = field_repo.find_all().map_err(|e| e.to_string())?;

    let field_infos: Vec<JiraFieldInfo> = fields
        .into_iter()
        .map(|f| JiraFieldInfo {
            id: f.id,
            name: f.name,
            custom: f.custom,
            navigable: f.navigable,
            schema_type: f.schema_type,
        })
        .collect();

    let total = field_infos.len() as i32;

    Ok(FieldsListResponse {
        fields: field_infos,
        total,
    })
}
