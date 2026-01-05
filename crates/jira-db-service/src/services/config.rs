//! Configuration service

use std::path::PathBuf;

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// Get current configuration
pub fn get(state: &AppState) -> ServiceResult<ConfigGetResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;

    Ok(ConfigGetResponse {
        settings: convert_settings(settings),
    })
}

/// Update configuration
pub fn update(
    state: &AppState,
    request: ConfigUpdateRequest,
) -> ServiceResult<ConfigUpdateResponse> {
    let updated = state
        .update_settings(|settings| {
            if let Some(jira) = request.jira {
                settings.jira.endpoint = jira.endpoint;
                settings.jira.username = jira.username;
                settings.jira.api_key = jira.api_key;
            }

            if let Some(database) = request.database {
                settings.database.database_dir = PathBuf::from(database.path);
            }

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
        .map_err(|e| ServiceError::Config(e.to_string()))?;

    Ok(ConfigUpdateResponse {
        success: true,
        settings: convert_settings(updated),
    })
}

/// Initialize configuration
pub fn initialize(
    state: &AppState,
    request: ConfigInitRequest,
) -> ServiceResult<ConfigInitResponse> {
    let settings_path = state
        .get_settings_path()
        .ok_or_else(|| ServiceError::Config("Settings path not configured".to_string()))?;

    // Determine database directory
    let database_dir = if let Some(db_path) = request.database_path {
        PathBuf::from(db_path)
    } else {
        if let Some(parent) = settings_path.parent() {
            parent.join("data")
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join("data"))
                .unwrap_or_else(|_| PathBuf::from("./data"))
        }
    };

    // Ensure the database directory exists
    std::fs::create_dir_all(&database_dir)
        .map_err(|e| ServiceError::Io(format!("Failed to create database directory: {}", e)))?;

    let settings = jira_db_core::Settings {
        jira: jira_db_core::JiraConfig {
            endpoint: request.endpoint,
            username: request.username,
            api_key: request.api_key,
        },
        projects: Vec::new(),
        database: jira_db_core::DatabaseConfig {
            path: None,
            database_dir,
        },
        embeddings: None,
        debug_mode: false,
    };

    state
        .create_settings(settings_path, settings.clone())
        .map_err(|e| ServiceError::Config(e.to_string()))?;

    Ok(ConfigInitResponse {
        success: true,
        settings: convert_settings(settings),
    })
}

/// Convert core Settings to API Settings
fn convert_settings(s: jira_db_core::Settings) -> Settings {
    Settings {
        jira: JiraConfig {
            endpoint: s.jira.endpoint,
            username: s.jira.username,
            api_key: s.jira.api_key,
        },
        database: DatabaseConfig {
            path: s.database.database_dir.to_string_lossy().to_string(),
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
