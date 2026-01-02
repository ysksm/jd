//! Config handlers

use std::sync::Arc;

use axum::Json;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Get current configuration
pub async fn get(
    state: Arc<AppState>,
    _request: ConfigGetRequest,
) -> Result<Json<ConfigGetResponse>, ApiError> {
    let settings = state.get_settings();

    Ok(Json(ConfigGetResponse {
        settings: Settings {
            jira: JiraConfig {
                endpoint: settings.jira.endpoint,
                username: settings.jira.username,
                api_key: "***".to_string(), // Mask API key
            },
            database: DatabaseConfig {
                path: settings.database.path,
            },
            projects: settings
                .projects
                .iter()
                .map(|p| ProjectConfig {
                    key: p.key.clone(),
                    enabled: p.enabled,
                })
                .collect(),
            embeddings: settings.embeddings.as_ref().map(|e| EmbeddingsConfig {
                provider: e.provider.clone(),
                model: e.model.clone(),
                endpoint: e.endpoint.clone(),
                auto_generate: e.auto_generate,
            }),
        },
    }))
}

/// Update configuration
pub async fn update(
    _state: Arc<AppState>,
    _request: ConfigUpdateRequest,
) -> Result<Json<ConfigUpdateResponse>, ApiError> {
    // TODO: Implement config update
    Err(ApiError::internal("Not implemented yet"))
}

/// Initialize configuration
pub async fn init(
    _state: Arc<AppState>,
    _request: ConfigInitRequest,
) -> Result<Json<ConfigInitResponse>, ApiError> {
    // TODO: Implement config initialization
    Err(ApiError::internal("Not implemented yet"))
}
