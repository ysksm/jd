//! Application state management
//!
//! Provides a shared state container for settings and database connection.

use std::path::PathBuf;
use std::sync::Mutex;

use jira_db_core::{Database, DbConnection, Settings, checkpoint_connection};

/// Default database filename
const DEFAULT_DB_FILENAME: &str = "jira.duckdb";

/// Shared application state
pub struct AppState {
    /// Path to settings file
    pub settings_path: Mutex<Option<PathBuf>>,
    /// Loaded settings
    pub settings: Mutex<Option<Settings>>,
    /// Database connection
    pub db: Mutex<Option<DbConnection>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings_path: Mutex::new(None),
            settings: Mutex::new(None),
            db: Mutex::new(None),
        }
    }
}

/// Ensure the database path is a file path, not a directory
fn ensure_db_file_path(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        return path.join(DEFAULT_DB_FILENAME);
    }

    if path.extension().is_none() && !path.to_string_lossy().ends_with(".duckdb") {
        let path_str = path.to_string_lossy();
        if path_str == "." || path_str == ".." || path_str.ends_with('/') {
            return path.join(DEFAULT_DB_FILENAME);
        }
    }

    path
}

impl AppState {
    /// Create a new AppState
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize the application state with a settings file
    pub fn initialize(&self, settings_path: PathBuf) -> anyhow::Result<()> {
        let mut settings = Settings::load(&settings_path)?;

        tracing::info!(
            "Loaded settings, database_dir from file: {:?}",
            settings.database.database_dir
        );

        // Resolve database directory relative to settings file directory if it's relative
        let db_dir = if settings.database.database_dir.is_relative() {
            if let Some(settings_dir) = settings_path.parent() {
                let resolved = settings_dir.join(&settings.database.database_dir);
                resolved.canonicalize().unwrap_or(resolved)
            } else {
                settings.database.database_dir.clone()
            }
        } else {
            settings.database.database_dir.clone()
        };

        // For service, we use a default database file in the database directory
        let db_path = ensure_db_file_path(db_dir.clone());

        tracing::info!("Database path (resolved): {:?}", db_path);

        // Initialize database with resolved path
        let db = Database::new(&db_path)?;

        // Update settings with resolved directory for consistency
        settings.database.database_dir = db_dir;

        // Store state
        *self.settings_path.lock().unwrap() = Some(settings_path);
        *self.db.lock().unwrap() = Some(db.connection());
        *self.settings.lock().unwrap() = Some(settings);

        Ok(())
    }

    /// Create new settings file and initialize
    pub fn create_settings(
        &self,
        settings_path: PathBuf,
        settings: Settings,
    ) -> anyhow::Result<()> {
        let mut settings = settings;

        // Resolve database directory relative to settings file directory if it's relative
        let db_dir = if settings.database.database_dir.is_relative() {
            if let Some(settings_dir) = settings_path.parent() {
                settings_dir.join(&settings.database.database_dir)
            } else {
                settings.database.database_dir.clone()
            }
        } else {
            settings.database.database_dir.clone()
        };

        // For service, we use a default database file in the database directory
        let db_path = ensure_db_file_path(db_dir.clone());

        tracing::info!("Database path (resolved): {:?}", db_path);

        // Update settings with resolved directory before saving
        settings.database.database_dir = db_dir;

        // Save settings with absolute path
        settings.save(&settings_path)?;

        // Initialize database
        let db = Database::new(&db_path)?;

        // Store state
        *self.settings_path.lock().unwrap() = Some(settings_path);
        *self.db.lock().unwrap() = Some(db.connection());
        *self.settings.lock().unwrap() = Some(settings);

        Ok(())
    }

    /// Update settings
    pub fn update_settings<F>(&self, update_fn: F) -> anyhow::Result<Settings>
    where
        F: FnOnce(&mut Settings),
    {
        let mut settings_guard = self.settings.lock().unwrap();
        let path_guard = self.settings_path.lock().unwrap();

        let settings = settings_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        let path = path_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No settings path"))?;

        // Apply updates
        update_fn(settings);

        // Save to disk
        settings.save(path)?;

        Ok(settings.clone())
    }

    /// Get a clone of current settings
    pub fn get_settings(&self) -> Option<Settings> {
        self.settings.lock().unwrap().clone()
    }

    /// Get settings path
    pub fn get_settings_path(&self) -> Option<PathBuf> {
        self.settings_path.lock().unwrap().clone()
    }

    /// Get database connection
    pub fn get_db(&self) -> Option<DbConnection> {
        self.db.lock().unwrap().clone()
    }

    /// Check if initialized
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.settings.lock().unwrap().is_some()
    }

    /// Cleanup the database connection by running a checkpoint.
    pub fn cleanup(&self) {
        if let Some(ref db) = *self.db.lock().unwrap() {
            tracing::info!("Running database checkpoint before exit...");
            match checkpoint_connection(db) {
                Ok(()) => {
                    tracing::info!("Database checkpoint completed successfully");
                }
                Err(e) => {
                    tracing::warn!("Failed to checkpoint database: {}", e);
                }
            }
        }
    }
}
