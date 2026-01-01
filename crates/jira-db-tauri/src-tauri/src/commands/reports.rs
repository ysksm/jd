//! Report command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Generate HTML report
#[tauri::command]
pub async fn reports_generate(
    _state: State<'_, AppState>,
    _request: ReportGenerateRequest,
) -> Result<ReportGenerateResponse, String> {
    Err("Not implemented".to_string())
}
