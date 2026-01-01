//! Issue command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{DuckDbIssueRepository, SearchIssuesUseCase, SearchParams};

use crate::generated::*;
use crate::state::AppState;

/// Search issues with filters
#[tauri::command]
pub async fn issues_search(
    state: State<'_, AppState>,
    request: IssueSearchRequest,
) -> Result<IssueSearchResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let use_case = SearchIssuesUseCase::new(issue_repo);

    let params = SearchParams {
        query: request.query,
        project_key: request.project,
        status: request.status,
        assignee: request.assignee,
        limit: request.limit.map(|l| l as usize),
        offset: request.offset.map(|o| o as usize),
    };

    let issues = use_case.execute(params).map_err(|e| e.to_string())?;
    let total = issues.len() as i32;

    let issues = issues
        .into_iter()
        .map(|i| Issue {
            id: i.id,
            key: i.key,
            project_key: i.project_key,
            summary: i.summary,
            description: i.description,
            status: i.status,
            priority: i.priority,
            issue_type: i.issue_type,
            assignee: i.assignee,
            reporter: i.reporter,
            labels: i.labels,
            components: i.components,
            fix_versions: i.fix_versions,
            created_at: i.created_date.to_rfc3339(),
            updated_at: i.updated_date.to_rfc3339(),
        })
        .collect();

    Ok(IssueSearchResponse { issues, total })
}

/// Get issue by key
#[tauri::command]
pub async fn issues_get(
    state: State<'_, AppState>,
    request: IssueGetRequest,
) -> Result<IssueGetResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));

    let issue = issue_repo
        .find_by_key(&request.key)
        .map_err(|e| e.to_string())?
        .ok_or("Issue not found")?;

    Ok(IssueGetResponse {
        issue: Issue {
            id: issue.id,
            key: issue.key,
            project_key: issue.project_key,
            summary: issue.summary,
            description: issue.description,
            status: issue.status,
            priority: issue.priority,
            issue_type: issue.issue_type,
            assignee: issue.assignee,
            reporter: issue.reporter,
            labels: issue.labels,
            components: issue.components,
            fix_versions: issue.fix_versions,
            created_at: issue.created_date.to_rfc3339(),
            updated_at: issue.updated_date.to_rfc3339(),
        },
    })
}

/// Get issue change history
#[tauri::command]
pub async fn issues_history(
    state: State<'_, AppState>,
    request: IssueHistoryRequest,
) -> Result<IssueHistoryResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));

    let history = issue_repo
        .get_change_history(&request.key, request.field.as_deref(), request.limit.map(|l| l as usize))
        .map_err(|e| e.to_string())?;

    let history = history
        .into_iter()
        .map(|h| ChangeHistoryItem {
            id: h.history_id,
            issue_key: h.issue_key,
            author: h.author_display_name,
            field: h.field,
            field_type: h.field_type,
            from_value: h.from_value,
            from_string: h.from_string,
            to_value: h.to_value,
            to_string: h.to_string,
            changed_at: h.changed_at.to_rfc3339(),
        })
        .collect();

    Ok(IssueHistoryResponse { history })
}
