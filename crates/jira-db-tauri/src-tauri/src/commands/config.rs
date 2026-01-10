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
            // Migrate legacy config if needed
            settings.migrate_legacy_config();

            // Handle endpoint operations first
            // Add new endpoint
            if let Some(ref new_ep) = request.add_endpoint {
                let exists = settings
                    .jira_endpoints
                    .iter()
                    .any(|e| e.name == new_ep.name);
                if !exists {
                    settings.jira_endpoints.push(jira_db_core::JiraEndpoint {
                        name: new_ep.name.clone(),
                        display_name: new_ep.display_name.clone(),
                        endpoint: new_ep.endpoint.clone(),
                        username: new_ep.username.clone(),
                        api_key: new_ep.api_key.clone(),
                    });
                    // Set as active if it's the first endpoint
                    if settings.active_endpoint.is_none() {
                        settings.active_endpoint = Some(new_ep.name.clone());
                    }
                }
            }

            // Remove endpoint
            if let Some(ref name_to_remove) = request.remove_endpoint {
                settings
                    .jira_endpoints
                    .retain(|e| &e.name != name_to_remove);
                // If active endpoint was removed, set to first available
                if settings.active_endpoint.as_ref() == Some(name_to_remove) {
                    settings.active_endpoint =
                        settings.jira_endpoints.first().map(|e| e.name.clone());
                }
            }

            // Set active endpoint
            if let Some(ref active_name) = request.set_active_endpoint {
                if settings
                    .jira_endpoints
                    .iter()
                    .any(|e| &e.name == active_name)
                {
                    settings.active_endpoint = Some(active_name.clone());
                }
            }

            // Update active endpoint's JIRA config if provided
            if let Some(jira) = request.jira.clone() {
                // Find active endpoint and update it, or create a default one
                if let Some(active_name) = &settings.active_endpoint {
                    if let Some(endpoint) = settings
                        .jira_endpoints
                        .iter_mut()
                        .find(|e| &e.name == active_name)
                    {
                        endpoint.endpoint = jira.endpoint;
                        endpoint.username = jira.username;
                        endpoint.api_key = jira.api_key;
                    }
                } else if settings.jira_endpoints.is_empty() {
                    // Create a default endpoint
                    let new_endpoint = jira_db_core::JiraEndpoint {
                        name: "default".to_string(),
                        display_name: Some("Default".to_string()),
                        endpoint: jira.endpoint,
                        username: jira.username,
                        api_key: jira.api_key,
                    };
                    settings.jira_endpoints.push(new_endpoint);
                    settings.active_endpoint = Some("default".to_string());
                }
            }

            // Update database config if provided
            if let Some(database) = request.database.clone() {
                settings.database.database_dir = PathBuf::from(database.path);
            }

            // Update embeddings config if provided
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

            // Update log config if provided
            if let Some(log) = request.log.clone() {
                settings.log = Some(jira_db_core::LogConfig {
                    file_enabled: log.file_enabled,
                    file_dir: log.file_dir.map(PathBuf::from),
                    level: log.level,
                    max_files: log.max_files as usize,
                });
            }

            // Update sync config if provided
            if let Some(sync) = request.sync.clone() {
                settings.sync = Some(jira_db_core::SyncSettings {
                    incremental_sync_enabled: sync.incremental_sync_enabled,
                    incremental_sync_margin_minutes: sync.incremental_sync_margin_minutes as u32,
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
pub async fn config_initialize(
    state: State<'_, AppState>,
    request: ConfigInitRequest,
) -> Result<ConfigInitResponse, String> {
    // Use the settings path set during app setup (./settings.json in current directory)
    let settings_path = state
        .get_settings_path()
        .ok_or("Settings path not configured. App may not be properly initialized.")?;

    // Determine database directory - use provided path or default relative to settings directory
    let database_dir = if let Some(db_path) = request.database_path {
        PathBuf::from(db_path)
    } else {
        // Default: put database in a 'data' subdirectory relative to settings file
        if let Some(parent) = settings_path.parent() {
            parent.join("data")
        } else {
            // Fallback to current directory with absolute path
            std::env::current_dir()
                .map(|cwd| cwd.join("data"))
                .unwrap_or_else(|_| PathBuf::from("./data"))
        }
    };

    // Ensure the database directory exists
    std::fs::create_dir_all(&database_dir)
        .map_err(|e| format!("Failed to create database directory: {}", e))?;

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
        .map_err(|e| e.to_string())?;

    Ok(ConfigInitResponse {
        success: true,
        settings: settings.into(),
    })
}

// Conversion from jira-db-core types to generated types
impl From<jira_db_core::Settings> for Settings {
    fn from(mut s: jira_db_core::Settings) -> Self {
        // Migrate legacy config to get active endpoint
        s.migrate_legacy_config();

        // Get the active endpoint's config, or create a placeholder
        let jira_config = s.get_jira_config().unwrap_or(jira_db_core::JiraConfig {
            endpoint: String::new(),
            username: String::new(),
            api_key: String::new(),
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

        // Get sync settings
        let sync_settings = s.get_sync_settings();

        Self {
            jira: JiraConfig {
                endpoint: jira_config.endpoint,
                username: jira_config.username,
                api_key: jira_config.api_key,
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
            log: s.log.map(|l| LogConfig {
                file_enabled: l.file_enabled,
                file_dir: l.file_dir.map(|p| p.to_string_lossy().to_string()),
                level: l.level,
                max_files: l.max_files as i32,
            }),
            sync: Some(SyncConfig {
                incremental_sync_enabled: sync_settings.incremental_sync_enabled,
                incremental_sync_margin_minutes: sync_settings.incremental_sync_margin_minutes
                    as i32,
            }),
            jira_endpoints: if jira_endpoints.is_empty() {
                None
            } else {
                Some(jira_endpoints)
            },
            active_endpoint: s.active_endpoint,
        }
    }
}
