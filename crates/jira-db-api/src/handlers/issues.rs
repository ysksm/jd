//! Issues handlers

use std::sync::Arc;

use axum::Json;
use jira_db_core::{DuckDbIssueRepository, IssueRepository, SearchParams};

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Search issues with filters
pub async fn search(
    state: Arc<AppState>,
    request: IssueSearchRequest,
) -> Result<Json<IssueSearchResponse>, ApiError> {
    let repo = DuckDbIssueRepository::new(state.db.clone());

    let params = SearchParams {
        query: request.query,
        project_key: request.project,
        status: request.status,
        assignee: request.assignee,
        issue_type: request.issue_type,
        priority: request.priority,
        limit: request.limit.map(|l| l as usize),
        offset: request.offset.map(|o| o as usize),
    };

    let issues = repo.search(&params)?;

    let api_issues: Vec<Issue> = issues
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
            created_at: i.created_at,
            updated_at: i.updated_at,
        })
        .collect();

    let total = api_issues.len() as i32;

    Ok(Json(IssueSearchResponse {
        issues: api_issues,
        total,
    }))
}

/// Get issue by key
pub async fn get(
    state: Arc<AppState>,
    request: IssueGetRequest,
) -> Result<Json<IssueGetResponse>, ApiError> {
    let repo = DuckDbIssueRepository::new(state.db.clone());

    let issue = repo
        .find_by_key(&request.key)?
        .ok_or_else(|| ApiError::not_found(format!("Issue not found: {}", request.key)))?;

    Ok(Json(IssueGetResponse {
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
            created_at: issue.created_at,
            updated_at: issue.updated_at,
        },
    }))
}

/// Get issue change history
pub async fn history(
    state: Arc<AppState>,
    request: IssueHistoryRequest,
) -> Result<Json<IssueHistoryResponse>, ApiError> {
    let repo = jira_db_core::DuckDbChangeHistoryRepository::new(state.db.clone());

    let history = repo.find_by_issue_key(
        &request.key,
        request.field.as_deref(),
        request.limit.map(|l| l as usize),
    )?;

    let api_history: Vec<ChangeHistoryItem> = history
        .into_iter()
        .map(|h| ChangeHistoryItem {
            id: h.id,
            issue_key: h.issue_key,
            author: h.author_display_name,
            field: h.field,
            field_type: h.field_type,
            from_value: h.from_value,
            from_string: h.from_string,
            to_value: h.to_value,
            to_string: h.to_string,
            changed_at: h.changed_at,
        })
        .collect();

    Ok(Json(IssueHistoryResponse {
        history: api_history,
    }))
}
