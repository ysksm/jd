//! Metadata command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Get project metadata
#[tauri::command]
pub async fn metadata_get(
    _state: State<'_, AppState>,
    _request: MetadataGetRequest,
) -> Result<MetadataGetResponse, String> {
    Err("Not implemented".to_string())
}
