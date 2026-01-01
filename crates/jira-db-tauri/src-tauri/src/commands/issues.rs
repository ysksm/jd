//! Issue command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Search issues with filters
#[tauri::command]
pub async fn issues_search(
    _state: State<'_, AppState>,
    _request: IssueSearchRequest,
) -> Result<IssueSearchResponse, String> {
    Err("Not implemented".to_string())
}

/// Get issue by key
#[tauri::command]
pub async fn issues_get(
    _state: State<'_, AppState>,
    _request: IssueGetRequest,
) -> Result<IssueGetResponse, String> {
    Err("Not implemented".to_string())
}

/// Get issue change history
#[tauri::command]
pub async fn issues_history(
    _state: State<'_, AppState>,
    _request: IssueHistoryRequest,
) -> Result<IssueHistoryResponse, String> {
    Err("Not implemented".to_string())
}
