use super::schema::Schema;
use crate::domain::error::{DomainError, DomainResult};
use crate::infrastructure::config::Settings;
use duckdb::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Type alias for the database connection handle
pub type DbConnection = Arc<Mutex<Connection>>;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> DomainResult<Self> {
        Self::new_with_schema(path, true)
    }

    /// Create a new database with optional schema initialization
    pub fn new_with_schema<P: AsRef<Path>>(path: P, init_main_schema: bool) -> DomainResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Repository(format!("Failed to create database directory: {}", e))
            })?;
        }

        let conn = Connection::open(path)
            .map_err(|e| DomainError::Repository(format!("Failed to open database: {}", e)))?;

        // Initialize schema
        if init_main_schema {
            Schema::init(&conn)?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a raw data database
    pub fn new_raw<P: AsRef<Path>>(path: P) -> DomainResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Repository(format!("Failed to create database directory: {}", e))
            })?;
        }

        let conn = Connection::open(path)
            .map_err(|e| DomainError::Repository(format!("Failed to open database: {}", e)))?;

        // Initialize raw data schema
        Schema::init_raw(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// Run a checkpoint to flush WAL file to the main database file.
    /// This should be called before closing the application to ensure
    /// all data is properly persisted and WAL file is removed.
    pub fn checkpoint(&self) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        conn.execute_batch("CHECKPOINT").map_err(|e| {
            DomainError::Repository(format!("Failed to checkpoint database: {}", e))
        })?;

        Ok(())
    }
}

/// Run checkpoint on a database connection.
/// This is a standalone function for use with DbConnection type.
pub fn checkpoint_connection(conn: &DbConnection) -> DomainResult<()> {
    let conn = conn
        .lock()
        .map_err(|e| DomainError::Repository(format!("Failed to acquire database lock: {}", e)))?;

    conn.execute_batch("CHECKPOINT")
        .map_err(|e| DomainError::Repository(format!("Failed to checkpoint database: {}", e)))?;

    Ok(())
}

/// Connection key for the connection cache
#[derive(Clone, Hash, PartialEq, Eq)]
struct ConnectionKey {
    project_key: String,
    is_raw: bool,
}

/// Factory for managing per-project database connections
///
/// Each project has its own subdirectory with separate database files:
/// - {database_dir}/{project_key}/data.duckdb - processed data
/// - {database_dir}/{project_key}/raw.duckdb - raw JSON data
pub struct DatabaseFactory {
    database_dir: PathBuf,
    connections: Arc<Mutex<HashMap<ConnectionKey, DbConnection>>>,
}

impl DatabaseFactory {
    /// Create a new database factory from settings
    pub fn new(settings: &Settings) -> Self {
        Self {
            database_dir: settings.database.database_dir.clone(),
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new database factory with a specific database directory
    pub fn with_dir<P: AsRef<Path>>(database_dir: P) -> Self {
        Self {
            database_dir: database_dir.as_ref().to_path_buf(),
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the project directory path
    fn get_project_dir(&self, project_key: &str) -> PathBuf {
        self.database_dir.join(project_key)
    }

    /// Get or create a database connection for a specific project (main data)
    pub fn get_connection(&self, project_key: &str) -> DomainResult<DbConnection> {
        let key = ConnectionKey {
            project_key: project_key.to_string(),
            is_raw: false,
        };

        let mut connections = self.connections.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connections lock: {}", e))
        })?;

        if let Some(conn) = connections.get(&key) {
            return Ok(conn.clone());
        }

        // Create new connection for this project
        let db_path = self.get_project_dir(project_key).join("data.duckdb");
        let db = Database::new(&db_path)?;
        let conn = db.connection();
        connections.insert(key, conn.clone());

        Ok(conn)
    }

    /// Get or create a raw data database connection for a specific project
    pub fn get_raw_connection(&self, project_key: &str) -> DomainResult<DbConnection> {
        let key = ConnectionKey {
            project_key: project_key.to_string(),
            is_raw: true,
        };

        let mut connections = self.connections.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connections lock: {}", e))
        })?;

        if let Some(conn) = connections.get(&key) {
            return Ok(conn.clone());
        }

        // Create new connection for raw data
        let db_path = self.get_project_dir(project_key).join("raw.duckdb");
        let db = Database::new_raw(&db_path)?;
        let conn = db.connection();
        connections.insert(key, conn.clone());

        Ok(conn)
    }

    /// Get the main database path for a specific project
    pub fn get_database_path(&self, project_key: &str) -> PathBuf {
        self.get_project_dir(project_key).join("data.duckdb")
    }

    /// Get the raw database path for a specific project
    pub fn get_raw_database_path(&self, project_key: &str) -> PathBuf {
        self.get_project_dir(project_key).join("raw.duckdb")
    }

    /// Get the database directory
    pub fn database_dir(&self) -> &Path {
        &self.database_dir
    }

    /// List all project databases in the directory
    pub fn list_project_databases(&self) -> DomainResult<Vec<String>> {
        let mut projects = Vec::new();

        if !self.database_dir.exists() {
            return Ok(projects);
        }

        let entries = std::fs::read_dir(&self.database_dir).map_err(|e| {
            DomainError::Repository(format!("Failed to read database directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                DomainError::Repository(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            // Check for subdirectory with data.duckdb
            if path.is_dir() {
                let data_db = path.join("data.duckdb");
                if data_db.exists() {
                    if let Some(name) = path.file_name() {
                        projects.push(name.to_string_lossy().to_string());
                    }
                }
            }
            // Also check for legacy flat .duckdb files
            else if path.extension().is_some_and(|ext| ext == "duckdb") {
                if let Some(stem) = path.file_stem() {
                    let stem_str = stem.to_string_lossy().to_string();
                    // Skip if there's already a subdirectory with the same name
                    if !self.database_dir.join(&stem_str).is_dir() {
                        projects.push(stem_str);
                    }
                }
            }
        }

        Ok(projects)
    }

    /// Checkpoint all open connections
    pub fn checkpoint_all(&self) -> DomainResult<()> {
        let connections = self.connections.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connections lock: {}", e))
        })?;

        for (key, conn) in connections.iter() {
            checkpoint_connection(conn).map_err(|e| {
                DomainError::Repository(format!(
                    "Failed to checkpoint database for project {} (raw={}): {}",
                    key.project_key, key.is_raw, e
                ))
            })?;
        }

        Ok(())
    }
}
