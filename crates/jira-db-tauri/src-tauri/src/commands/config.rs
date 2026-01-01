//! Configuration command handlers

use std::path::PathBuf;
use tauri::State;

use crate::generated::*;
use crate::state::AppState;

/// Get current configuration
#[tauri::command]
pub async fn config_get(
    state: State<'_, AppState>,
    _request: ConfigGetRequest,
) -> Result<ConfigGetResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;

    Ok(ConfigGetResponse {
        settings: settings.into(),
    })
}

/// Update configuration
#[tauri::command]
pub async fn config_update(
    state: State<'_, AppState>,
    request: ConfigUpdateRequest,
) -> Result<ConfigUpdateResponse, String> {
    let updated = state
        .update_settings(|settings| {
            // Update JIRA config if provided
            if let Some(jira) = request.jira {
                settings.jira.endpoint = jira.endpoint;
                settings.jira.username = jira.username;
                settings.jira.api_key = jira.api_key;
            }

            // Update database config if provided
            if let Some(database) = request.database {
                settings.database.path = PathBuf::from(database.path);
            }

            // Update embeddings config if provided
            if let Some(embeddings) = request.embeddings {
                settings.embeddings = Some(jira_db_core::EmbeddingsConfig {
                    provider: embeddings.provider,
                    api_key: None,
                    openai_api_key: None,
                    model: embeddings.model_name.unwrap_or_default(),
                    endpoint: embeddings.endpoint,
                    auto_generate: embeddings.auto_generate,
                });
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(ConfigUpdateResponse {
        success: true,
        settings: updated.into(),
    })
}

/// Initialize configuration
#[tauri::command]
pub async fn config_init(
    state: State<'_, AppState>,
    request: ConfigInitRequest,
) -> Result<ConfigInitResponse, String> {
    let settings_path = PathBuf::from("./settings.json");

    let database_path = request
        .database_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data/jira.duckdb"));

    let settings = jira_db_core::Settings {
        jira: jira_db_core::JiraConfig {
            endpoint: request.endpoint,
            username: request.username,
            api_key: request.api_key,
        },
        projects: Vec::new(),
        database: jira_db_core::DatabaseConfig { path: database_path },
        embeddings: None,
    };

    state
        .create_settings(settings_path, settings.clone())
        .map_err(|e| e.to_string())?;

    Ok(ConfigInitResponse {
        success: true,
        settings: settings.into(),
    })
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
                model_name: Some(e.model),
                endpoint: e.endpoint,
                auto_generate: e.auto_generate,
            }),
        }
    }
}
