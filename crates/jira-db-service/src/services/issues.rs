//! Issues service

use std::sync::Arc;

use jira_db_core::{
    ChangeHistoryRepository, DuckDbChangeHistoryRepository, DuckDbIssueRepository,
    SearchIssuesUseCase, SearchParams,
};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// Convert core Issue to API Issue type
fn convert_issue(i: jira_db_core::Issue) -> Issue {
    Issue {
        id: i.id,
        key: i.key.clone(),
        project_key: i.key.split('-').next().unwrap_or("").to_string(),
        summary: i.summary,
        description: i.description,
        status: i.status.unwrap_or_default(),
        priority: i.priority.unwrap_or_default(),
        issue_type: i.issue_type.unwrap_or_default(),
        assignee: i.assignee,
        reporter: i.reporter,
        labels: i.labels.unwrap_or_default(),
        components: i.components.unwrap_or_default(),
        fix_versions: i.fix_versions.unwrap_or_default(),
        created_at: i.created_date.unwrap_or_else(chrono::Utc::now),
        updated_at: i.updated_date.unwrap_or_else(chrono::Utc::now),
    }
}

/// Search issues with filters
pub fn search(state: &AppState, request: IssueSearchRequest) -> ServiceResult<IssueSearchResponse> {
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let use_case = SearchIssuesUseCase::new(issue_repo);

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

    let issues = use_case.execute(params)?;
    let total = issues.len() as i32;

    let issues = issues.into_iter().map(convert_issue).collect();

    Ok(IssueSearchResponse { issues, total })
}

/// Get issue by key
pub fn get(state: &AppState, request: IssueGetRequest) -> ServiceResult<IssueGetResponse> {
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let use_case = SearchIssuesUseCase::new(issue_repo);

    // Use search with the exact key as query
    let params = SearchParams {
        query: Some(request.key.clone()),
        project_key: None,
        status: None,
        assignee: None,
        issue_type: None,
        priority: None,
        limit: Some(1),
        offset: None,
    };

    let issues = use_case.execute(params)?;

    let issue = issues
        .into_iter()
        .find(|i| i.key == request.key)
        .ok_or_else(|| ServiceError::NotFound("Issue not found".to_string()))?;

    Ok(IssueGetResponse {
        issue: convert_issue(issue),
    })
}

/// Get issue change history
pub fn history(
    state: &AppState,
    request: IssueHistoryRequest,
) -> ServiceResult<IssueHistoryResponse> {
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    let history_repo = DuckDbChangeHistoryRepository::new(db);

    let history =
        history_repo.find_by_issue_key_and_field(&request.key, request.field.as_deref())?;

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
