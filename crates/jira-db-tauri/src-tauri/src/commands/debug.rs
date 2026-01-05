//! Debug command handlers for JIRA test data creation

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    JiraApiClient, JiraConfig,
    application::use_cases::{CreateTestTicketUseCase, TransitionIssueUseCase},
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// ============================================================
// Debug Request/Response Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStatusRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStatusResponse {
    pub enabled: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugCreateIssuesRequest {
    pub project: String,
    #[serde(default = "default_count")]
    pub count: usize,
    #[serde(rename = "issueType")]
    #[serde(default = "default_issue_type")]
    pub issue_type: String,
    #[serde(default = "default_summary")]
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_count() -> usize {
    1
}

fn default_issue_type() -> String {
    "Task".to_string()
}

fn default_summary() -> String {
    "[Debug] Test Issue".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedIssue {
    pub key: String,
    pub id: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugCreateIssuesResponse {
    pub success: bool,
    pub created: Vec<CreatedIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugListTransitionsRequest {
    #[serde(rename = "issueKey")]
    pub issue_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: String,
    pub name: String,
    #[serde(rename = "toStatus")]
    pub to_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugListTransitionsResponse {
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugTransitionIssueRequest {
    #[serde(rename = "issueKey")]
    pub issue_key: String,
    #[serde(rename = "transitionId")]
    pub transition_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugTransitionIssueResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugBulkTransitionRequest {
    pub issues: Vec<String>,
    #[serde(rename = "transitionId")]
    pub transition_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkTransitionResult {
    #[serde(rename = "issueKey")]
    pub issue_key: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugBulkTransitionResponse {
    pub results: Vec<BulkTransitionResult>,
    #[serde(rename = "successCount")]
    pub success_count: i32,
    #[serde(rename = "failureCount")]
    pub failure_count: i32,
}

// ============================================================
// Command Handlers
// ============================================================

/// Get debug mode status
#[tauri::command]
pub async fn debug_status(
    state: State<'_, AppState>,
    _request: DebugStatusRequest,
) -> Result<DebugStatusResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    Ok(DebugStatusResponse {
        enabled: settings.debug_mode,
        message: if settings.debug_mode {
            "Debug mode is enabled. JIRA test data creation tools are available.".to_string()
        } else {
            "Debug mode is disabled. Set debug_mode: true in settings.json to enable.".to_string()
        },
    })
}

/// Create test issues in JIRA
#[tauri::command]
pub async fn debug_create_issues(
    state: State<'_, AppState>,
    request: DebugCreateIssuesRequest,
) -> Result<DebugCreateIssuesResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Err(
            "Debug mode is not enabled. Set debug_mode: true in settings.json to enable."
                .to_string(),
        );
    }

    // Validate count
    if request.count == 0 || request.count > 100 {
        return Err("Count must be between 1 and 100".to_string());
    }

    // Create JIRA client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create use case
    let use_case = CreateTestTicketUseCase::new(jira_client);

    // Create issues
    let mut created = Vec::new();
    let mut last_error = None;

    for i in 1..=request.count {
        let summary = if request.count > 1 {
            format!("{} #{}", request.summary, i)
        } else {
            request.summary.clone()
        };

        match use_case
            .execute(
                &request.project,
                &summary,
                request.description.as_deref(),
                &request.issue_type,
            )
            .await
        {
            Ok(result) => {
                created.push(CreatedIssue {
                    key: result.key,
                    id: result.id,
                    self_url: result.self_url,
                });
            }
            Err(e) => {
                last_error = Some(e.to_string());
                break;
            }
        }
    }

    Ok(DebugCreateIssuesResponse {
        success: last_error.is_none() && !created.is_empty(),
        created,
        error: last_error,
    })
}

/// List available transitions for an issue
#[tauri::command]
pub async fn debug_list_transitions(
    state: State<'_, AppState>,
    request: DebugListTransitionsRequest,
) -> Result<DebugListTransitionsResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Err(
            "Debug mode is not enabled. Set debug_mode: true in settings.json to enable."
                .to_string(),
        );
    }

    // Create JIRA client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Get transitions
    let use_case = TransitionIssueUseCase::new(jira_client);
    let transitions = use_case
        .get_transitions(&request.issue_key)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DebugListTransitionsResponse {
        transitions: transitions
            .into_iter()
            .map(|t| Transition {
                id: t.id,
                name: t.name,
                to_status: t.to_status_name,
            })
            .collect(),
    })
}

/// Transition a single issue
#[tauri::command]
pub async fn debug_transition_issue(
    state: State<'_, AppState>,
    request: DebugTransitionIssueRequest,
) -> Result<DebugTransitionIssueResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Err(
            "Debug mode is not enabled. Set debug_mode: true in settings.json to enable."
                .to_string(),
        );
    }

    // Create JIRA client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Transition issue
    let use_case = TransitionIssueUseCase::new(jira_client);
    match use_case
        .transition(&request.issue_key, &request.transition_id)
        .await
    {
        Ok(()) => Ok(DebugTransitionIssueResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(DebugTransitionIssueResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

/// Bulk transition multiple issues
#[tauri::command]
pub async fn debug_bulk_transition(
    state: State<'_, AppState>,
    request: DebugBulkTransitionRequest,
) -> Result<DebugBulkTransitionResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Err(
            "Debug mode is not enabled. Set debug_mode: true in settings.json to enable."
                .to_string(),
        );
    }

    if request.issues.is_empty() {
        return Err("No issues provided".to_string());
    }

    // Create JIRA client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Transition each issue
    let use_case = TransitionIssueUseCase::new(jira_client);
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failure_count = 0;

    for issue_key in &request.issues {
        match use_case.transition(issue_key, &request.transition_id).await {
            Ok(()) => {
                success_count += 1;
                results.push(BulkTransitionResult {
                    issue_key: issue_key.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failure_count += 1;
                results.push(BulkTransitionResult {
                    issue_key: issue_key.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(DebugBulkTransitionResponse {
        results,
        success_count,
        failure_count,
    })
}
