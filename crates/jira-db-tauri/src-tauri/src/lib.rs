//! JiraDb Tauri Application
//!
//! Desktop application for JiraDb using Tauri framework.

mod commands;
mod generated;
pub mod logging;
mod state;

use state::AppState;
use std::path::PathBuf;
use tauri::{Manager, RunEvent};

/// Default settings file path (relative, will be resolved to absolute)
const DEFAULT_SETTINGS_FILE: &str = "./data/settings.json";

/// Resolve a relative path to an absolute path based on the executable's directory
/// or the current working directory.
fn resolve_data_path() -> PathBuf {
    // First, try to use the executable's directory (works for bundled apps)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // For macOS app bundles, go up from Contents/MacOS to the app bundle's parent
            #[cfg(target_os = "macos")]
            {
                // Check if we're in an app bundle (path contains .app/Contents/MacOS)
                let exe_dir_str = exe_dir.to_string_lossy();
                if exe_dir_str.contains(".app/Contents/MacOS") {
                    // Go up to the directory containing the .app bundle
                    if let Some(app_parent) = exe_dir
                        .parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.parent())
                    {
                        let settings_path = app_parent.join("data").join("settings.json");
                        if settings_path.exists() {
                            return settings_path;
                        }
                    }
                }
            }

            // For non-bundled apps, check relative to executable
            let settings_path = exe_dir.join("data").join("settings.json");
            if settings_path.exists() {
                return settings_path;
            }
        }
    }

    // Fall back to current working directory and resolve to absolute path
    let relative_path = PathBuf::from(DEFAULT_SETTINGS_FILE);
    if let Ok(cwd) = std::env::current_dir() {
        let absolute_path = cwd.join(&relative_path);
        // Canonicalize if the file exists, otherwise just use the joined path
        if absolute_path.exists() {
            absolute_path.canonicalize().unwrap_or(absolute_path)
        } else {
            absolute_path
        }
    } else {
        relative_path
    }
}

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt::init();

    // Bridge log crate to tracing (for jira-db-core logs)
    tracing_log::LogTracer::init().ok();

    // Initialize logging wrapper
    logging::init(logging::LogOutput::Console);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            // Resolve the settings path to an absolute path
            let settings_path = resolve_data_path();
            let state = app.state::<AppState>();

            tracing::info!("Settings path (resolved): {:?}", settings_path);

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
                        // Still store the path for later use
                        *state.settings_path.lock().unwrap() = Some(settings_path);
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
            // Fields
            commands::fields::fields_sync,
            commands::fields::fields_expand,
            commands::fields::fields_full,
            commands::fields::fields_list,
            // Reports
            commands::reports::reports_generate,
            // SQL
            commands::sql::sql_execute,
            commands::sql::sql_get_schema,
            commands::sql::sql_query_list,
            commands::sql::sql_query_save,
            commands::sql::sql_query_delete,
            // Database management
            commands::database::database_close,
            commands::database::database_status,
            // Debug (requires debug_mode in settings)
            commands::debug::debug_status,
            commands::debug::debug_create_issues,
            commands::debug::debug_list_transitions,
            commands::debug::debug_transition_issue,
            commands::debug::debug_bulk_transition,
            commands::debug::debug_get_issue_types,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                // Run cleanup before exit
                let state = app_handle.state::<AppState>();
                state.cleanup();
            }
        });
}
