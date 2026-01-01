//! Sync command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Execute sync for enabled projects
#[tauri::command]
pub async fn sync_execute(
    _state: State<'_, AppState>,
    _request: SyncExecuteRequest,
) -> Result<SyncExecuteResponse, String> {
    Err("Not implemented".to_string())
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
