//! JiraDb Tauri Application
//!
//! Desktop application for JiraDb using Tauri framework.

mod commands;
mod generated;
mod state;

use state::AppState;
use std::path::PathBuf;
use tauri::Manager;

/// Settings file path (same as CLI: ./settings.json in current directory)
const SETTINGS_FILE: &str = "./settings.json";

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let settings_path = PathBuf::from(SETTINGS_FILE);
            let state = app.state::<AppState>();

            tracing::info!("Settings path: {:?}", settings_path);

            // Try to load existing settings
            if settings_path.exists() {
                match state.initialize(settings_path.clone()) {
                    Ok(()) => {
                        tracing::info!("Loaded existing settings from {:?}", settings_path);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load settings: {}. Will need to reinitialize.",
                            e
                        );
                    }
                }
            } else {
                tracing::info!(
                    "No settings file found at {:?}. Waiting for initialization.",
                    settings_path
                );
                // Store the path for later use when initializing
                *state.settings_path.lock().unwrap() = Some(settings_path);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Config
            commands::config::config_get,
            commands::config::config_update,
            commands::config::config_initialize,
            // Projects
            commands::projects::projects_list,
            commands::projects::projects_initialize,
            commands::projects::projects_enable,
            commands::projects::projects_disable,
            // Sync
            commands::sync::sync_execute,
            commands::sync::sync_status,
            // Issues
            commands::issues::issues_search,
            commands::issues::issues_get,
            commands::issues::issues_history,
            // Metadata
            commands::metadata::metadata_get,
            // Embeddings
            commands::embeddings::embeddings_generate,
            commands::embeddings::embeddings_search,
            // Reports
            commands::reports::reports_generate,
            // SQL
            commands::sql::sql_execute,
            commands::sql::sql_get_schema,
            commands::sql::sql_query_list,
            commands::sql::sql_query_save,
            commands::sql::sql_query_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
