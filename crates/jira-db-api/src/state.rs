//! Application state management

use std::path::Path;
use std::sync::{Arc, Mutex};

use jira_db_core::{Database, DbConnection, Settings};

/// Shared application state
pub struct AppState {
    /// Path to settings file
    pub settings_path: String,
    /// Loaded settings (can be reloaded)
    pub settings: Mutex<Settings>,
    /// Database connection
    pub db: DbConnection,
}

impl AppState {
    /// Create new application state
    pub fn new(settings_path: &str) -> anyhow::Result<Self> {
        let settings = if Path::new(settings_path).exists() {
            Settings::load(Path::new(settings_path))?
        } else {
            Settings::default()
        };

        let db = if Path::new(&settings.database.path).exists() {
            let database = Database::new(&settings.database.path)?;
            database.connection()
        } else {
            // Create in-memory database if file doesn't exist
            let database = Database::new(":memory:")?;
            database.connection()
        };

        Ok(Self {
            settings_path: settings_path.to_string(),
            settings: Mutex::new(settings),
            db,
        })
    }

    /// Reload settings from disk
    pub fn reload_settings(&self) -> anyhow::Result<()> {
        let settings = Settings::load(Path::new(&self.settings_path))?;
        let mut guard = self.settings.lock().unwrap();
        *guard = settings;
        Ok(())
    }

    /// Get a clone of current settings
    pub fn get_settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}
