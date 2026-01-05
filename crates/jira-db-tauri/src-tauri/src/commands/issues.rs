//! Issue command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    ChangeHistoryRepository, DuckDbChangeHistoryRepository, DuckDbIssueRepository,
    SearchIssuesUseCase, SearchParams,
};

use crate::generated::*;
use crate::state::AppState;

/// Extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
fn extract_project_key(issue_key: &str) -> Option<&str> {
    issue_key.split('-').next()
}

/// Convert core Issue to generated Issue type
fn convert_issue(i: jira_db_core::Issue) -> Issue {
    Issue {
        id: i.id,
        key: i.key.clone(),
        // Use the key's project prefix since core has project_id, not project_key
        project_key: i.key.split('-').next().unwrap_or("").to_string(),
        summary: i.summary,
        description: i.description,
        status: i.status.unwrap_or_default(),
        priority: i.priority.unwrap_or_default(),
        issue_type: i.issue_type.unwrap_or_default(),
        assignee: i.assignee,
        reporter: i.reporter,
        parent_key: i.parent_key,
        labels: i.labels.unwrap_or_default(),
        components: i.components.unwrap_or_default(),
        fix_versions: i.fix_versions.unwrap_or_default(),
        created_at: i.created_date.unwrap_or_else(chrono::Utc::now),
        updated_at: i.updated_date.unwrap_or_else(chrono::Utc::now),
        due_date: i.due_date,
    }
}

/// Search issues with filters
#[tauri::command]
pub async fn issues_search(
    state: State<'_, AppState>,
    request: IssueSearchRequest,
) -> Result<IssueSearchResponse, String> {
    // Project key is required for per-project database
    let project_key = request
        .project
        .as_ref()
        .ok_or("project is required for search (per-project database)")?;

    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let use_case = SearchIssuesUseCase::new(issue_repo);

    let params = SearchParams {
        query: request.query,
        project_key: request.project.clone(),
        status: request.status,
        assignee: request.assignee,
        issue_type: request.issue_type,
        priority: request.priority,
        limit: request.limit.map(|l| l as usize),
        offset: request.offset.map(|o| o as usize),
    };

    let issues = use_case.execute(params).map_err(|e| e.to_string())?;
    let total = issues.len() as i32;

    let issues = issues.into_iter().map(convert_issue).collect();

    Ok(IssueSearchResponse { issues, total })
}

/// Get issue by key
#[tauri::command]
pub async fn issues_get(
    state: State<'_, AppState>,
    request: IssueGetRequest,
) -> Result<IssueGetResponse, String> {
    // Extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
    let project_key = extract_project_key(&request.key)
        .ok_or_else(|| format!("Invalid issue key format: {}", request.key))?;

    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let use_case = SearchIssuesUseCase::new(issue_repo);

    // Use search with the exact key as query
    let params = SearchParams {
        query: Some(request.key.clone()),
        project_key: Some(project_key.to_string()),
        status: None,
        assignee: None,
        issue_type: None,
        priority: None,
        limit: Some(1),
        offset: None,
    };

    let issues = use_case.execute(params).map_err(|e| e.to_string())?;

    let issue = issues
        .into_iter()
        .find(|i| i.key == request.key)
        .ok_or("Issue not found")?;

    Ok(IssueGetResponse {
        issue: convert_issue(issue),
    })
}

/// Get issue change history
#[tauri::command]
pub async fn issues_history(
    state: State<'_, AppState>,
    request: IssueHistoryRequest,
) -> Result<IssueHistoryResponse, String> {
    // Extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
    let project_key = extract_project_key(&request.key)
        .ok_or_else(|| format!("Invalid issue key format: {}", request.key))?;

    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    let history_repo = DuckDbChangeHistoryRepository::new(db);

    let history = history_repo
        .find_by_issue_key_and_field(&request.key, request.field.as_deref())
        .map_err(|e| e.to_string())?;

    // Apply limit if specified
    let history: Vec<_> = if let Some(limit) = request.limit {
        history.into_iter().take(limit as usize).collect()
    } else {
        history
    };

    let history = history
        .into_iter()
        .map(|h| ChangeHistoryItem {
            id: h.history_id,
            issue_key: h.issue_key,
            author: h.author_display_name.unwrap_or_default(),
            field: h.field,
            field_type: h.field_type.unwrap_or_default(),
            from_value: h.from_value,
            from_string: h.from_string,
            to_value: h.to_value,
            to_string: h.to_string,
            changed_at: h.changed_at,
        })
        .collect();

    Ok(IssueHistoryResponse { history })
}
