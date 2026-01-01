//! Configuration command handlers

use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Get current configuration
#[tauri::command]
pub async fn config_get(
    state: State<'_, AppState>,
    _request: ConfigGetRequest,
) -> Result<ConfigGetResponse, String> {
    let settings = state
        .get_settings()
        .ok_or("Not initialized")?;

    Ok(ConfigGetResponse {
        settings: settings.into(),
    })
}

/// Update configuration
#[tauri::command]
pub async fn config_update(
    _state: State<'_, AppState>,
    _request: ConfigUpdateRequest,
) -> Result<ConfigUpdateResponse, String> {
    Err("Not implemented".to_string())
}

/// Initialize configuration
#[tauri::command]
pub async fn config_init(
    _state: State<'_, AppState>,
    _request: ConfigInitRequest,
) -> Result<ConfigInitResponse, String> {
    Err("Not implemented".to_string())
}

// Conversion from jira-db-core types to generated types
impl From<jira_db_core::Settings> for Settings {
    fn from(s: jira_db_core::Settings) -> Self {
        Self {
            jira: JiraConfig {
                endpoint: s.jira.endpoint,
                username: s.jira.username,
                api_key: s.jira.api_key,
            },
            database: DatabaseConfig {
                path: s.database.path.to_string_lossy().to_string(),
            },
            projects: s
                .projects
                .into_iter()
                .map(|p| ProjectConfig {
                    key: p.key,
                    enabled: p.sync_enabled,
                })
                .collect(),
            embeddings: s.embeddings.map(|e| EmbeddingsConfig {
                provider: e.provider,
                model_name: e.model,
                endpoint: e.endpoint,
                auto_generate: e.auto_generate,
            }),
        }
    }
}
