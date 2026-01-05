//! Database management command handlers

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// Request to close database connections for a project
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCloseRequest {
    /// Project key to close connections for. If not specified, closes all connections.
    pub project_key: Option<String>,
}

/// Response for database close operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCloseResponse {
    pub success: bool,
    /// Number of connections still open after the operation
    pub open_connections: i32,
}

/// Request to get database status
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatusRequest {}

/// Response for database status
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatusResponse {
    /// Number of currently open connections
    pub open_connections: i32,
}

/// Close database connections
///
/// If project_key is specified, closes only that project's connections.
/// Otherwise, closes all connections.
#[tauri::command]
pub async fn database_close(
    state: State<'_, AppState>,
    request: DatabaseCloseRequest,
) -> Result<DatabaseCloseResponse, String> {
    if let Some(project_key) = request.project_key {
        state.close_db(&project_key)?;
        tracing::info!("Closed database connections for project {}", project_key);
    } else {
        state.close_all_dbs()?;
        tracing::info!("Closed all database connections");
    }

    Ok(DatabaseCloseResponse {
        success: true,
        open_connections: state.open_db_count() as i32,
    })
}

/// Get database connection status
#[tauri::command]
pub async fn database_status(
    state: State<'_, AppState>,
    _request: DatabaseStatusRequest,
) -> Result<DatabaseStatusResponse, String> {
    Ok(DatabaseStatusResponse {
        open_connections: state.open_db_count() as i32,
    })
}
