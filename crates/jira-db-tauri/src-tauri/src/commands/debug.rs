//! Debug command handlers for JIRA test data creation

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    AiTestDataConfig, GenerateAiTestDataUseCase, JiraApiClient, JiraConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugGetIssueTypesRequest {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueTypeInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub subtask: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugGetIssueTypesResponse {
    pub issue_types: Vec<IssueTypeInfo>,
}

// ============================================================
// AI Test Data Generation Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAiStatusRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAiStatusResponse {
    pub configured: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAiGenerateRequest {
    pub project: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprint_duration_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_transitions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epic_theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bug_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_fast_model: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedIssue {
    pub issue_type: String,
    pub summary: String,
    pub description: String,
    pub priority: String,
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub story_points: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_key: Option<String>,
    pub created_day_offset: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_day_offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_day_offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintScenario {
    pub sprint_name: String,
    pub duration_days: i32,
    pub team_members: Vec<String>,
    pub issues: Vec<GeneratedIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCreatedIssueInfo {
    pub key: String,
    pub id: String,
    pub issue_type: String,
    pub summary: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiFailedIssueInfo {
    pub issue_type: String,
    pub summary: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerationStats {
    pub total_generated: usize,
    pub successfully_created: usize,
    pub failed_to_create: usize,
    pub epics_created: usize,
    pub stories_created: usize,
    pub tasks_created: usize,
    pub bugs_created: usize,
    pub transitions_applied: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAiGenerateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<SprintScenario>,
    pub created_issues: Vec<AiCreatedIssueInfo>,
    pub failed_issues: Vec<AiFailedIssueInfo>,
    pub stats: AiGenerationStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
                    self_url: result.self_url.unwrap_or_default(),
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
                to_status: t.to_status,
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

/// Get available issue types for a project
#[tauri::command]
pub async fn debug_get_issue_types(
    state: State<'_, AppState>,
    request: DebugGetIssueTypesRequest,
) -> Result<DebugGetIssueTypesResponse, String> {
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
    let jira_client = JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?;

    // Fetch issue types
    use jira_db_core::application::services::JiraService;
    let issue_types = jira_client
        .fetch_issue_types_by_project_key(&request.project)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DebugGetIssueTypesResponse {
        issue_types: issue_types
            .into_iter()
            .map(|t| IssueTypeInfo {
                name: t.name,
                description: t.description,
                icon_url: t.icon_url,
                subtask: t.subtask,
            })
            .collect(),
    })
}

/// Check AI test data generation status
#[tauri::command]
pub async fn debug_ai_status(
    state: State<'_, AppState>,
    _request: DebugAiStatusRequest,
) -> Result<DebugAiStatusResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Ok(DebugAiStatusResponse {
            configured: false,
            message: "Debug mode is not enabled. Set debug_mode: true in settings.json."
                .to_string(),
        });
    }

    // Check for Anthropic API key
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

    if api_key.is_some() && !api_key.as_ref().unwrap().is_empty() {
        Ok(DebugAiStatusResponse {
            configured: true,
            message: "AI test data generation is ready. ANTHROPIC_API_KEY is configured."
                .to_string(),
        })
    } else {
        Ok(DebugAiStatusResponse {
            configured: false,
            message: "ANTHROPIC_API_KEY environment variable is not set. Set it to enable AI test data generation.".to_string(),
        })
    }
}

/// Generate test data using AI (Claude)
#[tauri::command]
pub async fn debug_ai_generate(
    state: State<'_, AppState>,
    request: DebugAiGenerateRequest,
) -> Result<DebugAiGenerateResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    // Check debug mode
    if !settings.debug_mode {
        return Err(
            "Debug mode is not enabled. Set debug_mode: true in settings.json to enable."
                .to_string(),
        );
    }

    // Get API key
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable is not set")?;

    if api_key.is_empty() {
        return Err("ANTHROPIC_API_KEY is empty".to_string());
    }

    // Create JIRA client
    let jira_config = JiraConfig {
        endpoint: settings.jira.endpoint.clone(),
        username: settings.jira.username.clone(),
        api_key: settings.jira.api_key.clone(),
    };
    let jira_client = Arc::new(JiraApiClient::new(&jira_config).map_err(|e| e.to_string())?);

    // Create use case
    let use_case = GenerateAiTestDataUseCase::new(jira_client);

    // Build config
    let config = AiTestDataConfig {
        project_context: request.project_context.unwrap_or_else(|| {
            "A software development project with web application components".to_string()
        }),
        team_size: request.team_size.unwrap_or(4),
        sprint_duration_days: request.sprint_duration_days.unwrap_or(14),
        apply_transitions: request.apply_transitions.unwrap_or(true),
        anthropic_api_key: api_key,
        use_fast_model: request.use_fast_model.unwrap_or(false),
    };

    // Execute based on mode
    let result = match request.mode.as_str() {
        "sprint" => use_case.execute(&request.project, &config).await,
        "epic" => {
            let theme = request
                .epic_theme
                .unwrap_or_else(|| "New Feature".to_string());
            use_case
                .generate_epic(&request.project, &config, &theme)
                .await
        }
        "bugs" => {
            let count = request.bug_count.unwrap_or(5);
            use_case
                .generate_bugs(&request.project, &config, count)
                .await
        }
        _ => {
            return Err(format!(
                "Unknown mode: {}. Use 'sprint', 'epic', or 'bugs'.",
                request.mode
            ));
        }
    };

    match result {
        Ok(data) => {
            // Convert to response types
            let scenario = Some(SprintScenario {
                sprint_name: data.scenario.sprint_name,
                duration_days: data.scenario.duration_days,
                team_members: data.scenario.team_members,
                issues: data
                    .scenario
                    .issues
                    .into_iter()
                    .map(|i| GeneratedIssue {
                        issue_type: i.issue_type,
                        summary: i.summary,
                        description: i.description,
                        priority: i.priority,
                        labels: i.labels,
                        story_points: i.story_points,
                        parent_key: i.parent_key,
                        created_day_offset: i.created_day_offset,
                        started_day_offset: i.started_day_offset,
                        completed_day_offset: i.completed_day_offset,
                        assignee: i.assignee,
                    })
                    .collect(),
            });

            let created_issues = data
                .created_issues
                .into_iter()
                .map(|i| AiCreatedIssueInfo {
                    key: i.key,
                    id: i.id,
                    issue_type: i.issue_type,
                    summary: i.summary,
                    status: i.status,
                    self_url: i.self_url,
                })
                .collect();

            let failed_issues = data
                .failed_issues
                .into_iter()
                .map(|i| AiFailedIssueInfo {
                    issue_type: i.issue_type,
                    summary: i.summary,
                    error: i.error,
                })
                .collect();

            let stats = AiGenerationStats {
                total_generated: data.stats.total_generated,
                successfully_created: data.stats.successfully_created,
                failed_to_create: data.stats.failed_to_create,
                epics_created: data.stats.epics_created,
                stories_created: data.stats.stories_created,
                tasks_created: data.stats.tasks_created,
                bugs_created: data.stats.bugs_created,
                transitions_applied: data.stats.transitions_applied,
            };

            Ok(DebugAiGenerateResponse {
                success: true,
                scenario,
                created_issues,
                failed_issues,
                stats,
                error: None,
            })
        }
        Err(e) => Ok(DebugAiGenerateResponse {
            success: false,
            scenario: None,
            created_issues: vec![],
            failed_issues: vec![],
            stats: AiGenerationStats {
                total_generated: 0,
                successfully_created: 0,
                failed_to_create: 0,
                epics_created: 0,
                stories_created: 0,
                tasks_created: 0,
                bugs_created: 0,
                transitions_applied: 0,
            },
            error: Some(e.to_string()),
        }),
    }
}
