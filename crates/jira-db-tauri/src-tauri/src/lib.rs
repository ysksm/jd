//! JiraDb Tauri Application
//!
//! Desktop application for JiraDb using Tauri framework.

mod commands;
mod generated;
mod state;

use state::AppState;

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // Config
            commands::config::config_get,
            commands::config::config_update,
            commands::config::config_init,
            // Projects
            commands::projects::projects_list,
            commands::projects::projects_init,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
