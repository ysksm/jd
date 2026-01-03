//! Application state management

use std::path::PathBuf;
use std::sync::Mutex;

use jira_db_core::{Database, DbConnection, Settings};

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

impl AppState {
    /// Initialize the application state with a settings file
    pub fn initialize(&self, settings_path: PathBuf) -> anyhow::Result<()> {
        let mut settings = Settings::load(&settings_path)?;

        // Resolve database path relative to settings file directory if it's relative
        let db_path = if settings.database.path.is_relative() {
            if let Some(settings_dir) = settings_path.parent() {
                let resolved = settings_dir.join(&settings.database.path);
                // Canonicalize if possible, otherwise use resolved path
                resolved.canonicalize().unwrap_or(resolved)
            } else {
                settings.database.path.clone()
            }
        } else {
            settings.database.path.clone()
        };

        tracing::info!("Database path (resolved): {:?}", db_path);

        // Initialize database with resolved path
        let db = Database::new(&db_path)?;

        // Update settings with resolved path for consistency
        settings.database.path = db_path;

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

        // Resolve database path relative to settings file directory if it's relative
        let db_path = if settings.database.path.is_relative() {
            if let Some(settings_dir) = settings_path.parent() {
                settings_dir.join(&settings.database.path)
            } else {
                settings.database.path.clone()
            }
        } else {
            settings.database.path.clone()
        };

        tracing::info!("Database path (resolved): {:?}", db_path);

        // Update settings with resolved path before saving
        settings.database.path = db_path.clone();

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
}
