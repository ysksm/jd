//! Application state management

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use jira_db_core::{DatabaseFactory, DbConnection, Settings};

/// Shared application state
pub struct AppState {
    /// Path to settings file
    pub settings_path: Mutex<Option<PathBuf>>,
    /// Loaded settings
    pub settings: Mutex<Option<Settings>>,
    /// Database factory for per-project databases
    pub db_factory: Mutex<Option<Arc<DatabaseFactory>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings_path: Mutex::new(None),
            settings: Mutex::new(None),
            db_factory: Mutex::new(None),
        }
    }
}

impl AppState {
    /// Initialize the application state with a settings file
    pub fn initialize(&self, settings_path: PathBuf) -> anyhow::Result<()> {
        // Load and resolve paths relative to settings file location
        let settings = Settings::load_and_resolve(&settings_path)?;

        tracing::info!(
            "Loaded settings, database_dir: {:?}",
            settings.database.database_dir
        );

        // Create database factory
        let db_factory = Arc::new(DatabaseFactory::new(&settings));

        // Store state
        *self.settings_path.lock().unwrap() = Some(settings_path);
        *self.db_factory.lock().unwrap() = Some(db_factory);
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

        // Resolve paths relative to settings file location
        settings.resolve_paths(&settings_path)?;

        tracing::info!(
            "Database directory (resolved): {:?}",
            settings.database.database_dir
        );

        // Ensure the database directory exists
        std::fs::create_dir_all(&settings.database.database_dir)?;

        // Save settings
        settings.save(&settings_path)?;

        // Create database factory
        let db_factory = Arc::new(DatabaseFactory::new(&settings));

        // Store state
        *self.settings_path.lock().unwrap() = Some(settings_path);
        *self.db_factory.lock().unwrap() = Some(db_factory);
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

    /// Get database factory
    pub fn get_db_factory(&self) -> Option<Arc<DatabaseFactory>> {
        self.db_factory.lock().unwrap().clone()
    }

    /// Get database connection for a specific project
    pub fn get_db(&self, project_key: &str) -> Option<DbConnection> {
        let factory = self.db_factory.lock().unwrap();
        factory
            .as_ref()
            .and_then(|f| f.get_connection(project_key).ok())
    }

    /// Get raw database connection for a specific project
    pub fn get_raw_db(&self, project_key: &str) -> Option<DbConnection> {
        let factory = self.db_factory.lock().unwrap();
        factory
            .as_ref()
            .and_then(|f| f.get_raw_connection(project_key).ok())
    }

    /// Check if initialized
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.settings.lock().unwrap().is_some()
    }

    /// Cleanup the database connections by running checkpoints.
    /// This flushes the WAL files to the main database files.
    /// Should be called before the application exits.
    pub fn cleanup(&self) {
        if let Some(ref factory) = *self.db_factory.lock().unwrap() {
            tracing::info!("Running database checkpoint before exit...");
            match factory.checkpoint_all() {
                Ok(()) => {
                    tracing::info!("Database checkpoint completed successfully");
                }
                Err(e) => {
                    tracing::warn!("Failed to checkpoint database: {}", e);
                }
            }
        }
    }

    /// Close database connections for a specific project
    /// This checkpoints and releases the connections
    pub fn close_db(&self, project_key: &str) -> Result<(), String> {
        if let Some(ref factory) = *self.db_factory.lock().unwrap() {
            factory
                .close_project(project_key)
                .map_err(|e| e.to_string())?;
            tracing::debug!("Closed database connections for project {}", project_key);
        }
        Ok(())
    }

    /// Close all database connections
    pub fn close_all_dbs(&self) -> Result<(), String> {
        if let Some(ref factory) = *self.db_factory.lock().unwrap() {
            factory.close_all().map_err(|e| e.to_string())?;
            tracing::debug!("Closed all database connections");
        }
        Ok(())
    }

    /// Get the number of open database connections
    pub fn open_db_count(&self) -> usize {
        self.db_factory
            .lock()
            .ok()
            .and_then(|f| f.as_ref().map(|factory| factory.open_connection_count()))
            .unwrap_or(0)
    }
}
