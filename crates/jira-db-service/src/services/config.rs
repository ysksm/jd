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
            // Handle JIRA config update (updates the active endpoint)
            if let Some(jira) = request.jira.clone() {
                if let Some(active_name) = &settings.active_endpoint {
                    // Find and update the active endpoint
                    if let Some(endpoint) = settings
                        .jira_endpoints
                        .iter_mut()
                        .find(|e| &e.name == active_name)
                    {
                        endpoint.endpoint = jira.endpoint;
                        endpoint.username = jira.username;
                        endpoint.api_key = jira.api_key;
                    }
                } else if let Some(endpoint) = settings.jira_endpoints.first_mut() {
                    // Update first endpoint if no active is set
                    endpoint.endpoint = jira.endpoint;
                    endpoint.username = jira.username;
                    endpoint.api_key = jira.api_key;
                } else {
                    // No endpoints exist, create a default one
                    settings.jira_endpoints.push(jira_db_core::JiraEndpoint {
                        name: "default".to_string(),
                        display_name: Some("Default".to_string()),
                        endpoint: jira.endpoint,
                        username: jira.username,
                        api_key: jira.api_key,
                    });
                    settings.active_endpoint = Some("default".to_string());
                }
            }

            // Handle add endpoint
            if let Some(ep) = request.add_endpoint.clone() {
                settings.add_endpoint(jira_db_core::JiraEndpoint {
                    name: ep.name,
                    display_name: ep.display_name,
                    endpoint: ep.endpoint,
                    username: ep.username,
                    api_key: ep.api_key,
                });
            }

            // Handle remove endpoint
            if let Some(name) = request.remove_endpoint.clone() {
                settings.remove_endpoint(&name);
            }

            // Handle set active endpoint
            if let Some(name) = request.set_active_endpoint.clone() {
                settings.set_active_endpoint(&name);
            }

            if let Some(database) = request.database.clone() {
                settings.database.database_dir = PathBuf::from(database.path);
            }

            if let Some(embeddings) = request.embeddings.clone() {
                settings.embeddings = Some(jira_db_core::EmbeddingsConfig {
                    provider: embeddings.provider,
                    api_key: None,
                    openai_api_key: None,
                    model: embeddings.model_name.unwrap_or_default(),
                    endpoint: embeddings.endpoint,
                    auto_generate: embeddings.auto_generate,
                });
            }

            if let Some(log) = request.log.clone() {
                settings.log = Some(jira_db_core::LogConfig {
                    file_enabled: log.file_enabled,
                    file_dir: log.file_dir.map(std::path::PathBuf::from),
                    level: log.level,
                    max_files: log.max_files as usize,
                });
            }

            if let Some(sync) = request.sync.clone() {
                settings.sync = Some(jira_db_core::SyncSettings {
                    incremental_sync_enabled: sync.incremental_sync_enabled,
                    incremental_sync_margin_minutes: sync.incremental_sync_margin_minutes as u32,
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
    } else if let Some(parent) = settings_path.parent() {
        parent.join("data")
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join("data"))
            .unwrap_or_else(|_| PathBuf::from("./data"))
    };

    // Ensure the database directory exists
    std::fs::create_dir_all(&database_dir)
        .map_err(|e| ServiceError::Io(format!("Failed to create database directory: {}", e)))?;

    let settings = jira_db_core::Settings::new(
        jira_db_core::JiraConfig {
            endpoint: request.endpoint,
            username: request.username,
            api_key: request.api_key,
        },
        database_dir,
    );

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
    let sync_settings = s.get_sync_settings();

    // Get effective JIRA config from active endpoint
    let jira = s.get_jira_config().map(|c| JiraConfig {
        endpoint: c.endpoint,
        username: c.username,
        api_key: c.api_key,
    });

    // Convert endpoints
    let jira_endpoints: Vec<JiraEndpoint> = s
        .jira_endpoints
        .iter()
        .map(|e| JiraEndpoint {
            name: e.name.clone(),
            display_name: e.display_name.clone(),
            endpoint: e.endpoint.clone(),
            username: e.username.clone(),
            api_key: e.api_key.clone(),
        })
        .collect();

    Settings {
        jira,
        jira_endpoints,
        active_endpoint: s.active_endpoint.clone(),
        database: DatabaseConfig {
            path: s.database.database_dir.to_string_lossy().to_string(),
        },
        projects: s
            .projects
            .into_iter()
            .map(|p| ProjectConfig {
                key: p.key,
                enabled: p.sync_enabled,
                endpoint: p.endpoint,
            })
            .collect(),
        embeddings: s.embeddings.map(|e| EmbeddingsConfig {
            provider: e.provider,
            model_name: Some(e.model),
            endpoint: e.endpoint,
            auto_generate: e.auto_generate,
        }),
        log: s.log.map(|l| LogConfig {
            file_enabled: l.file_enabled,
            file_dir: l.file_dir.map(|p| p.to_string_lossy().to_string()),
            level: l.level,
            max_files: l.max_files as i32,
        }),
        sync: Some(SyncConfig {
            incremental_sync_enabled: sync_settings.incremental_sync_enabled,
            incremental_sync_margin_minutes: sync_settings.incremental_sync_margin_minutes as i32,
        }),
    }
}
