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
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Repository(format!("Failed to create database directory: {}", e))
            })?;
        }

        let conn = Connection::open(path)
            .map_err(|e| DomainError::Repository(format!("Failed to open database: {}", e)))?;

        // Initialize schema
        Schema::init(&conn)?;

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

/// Factory for managing per-project database connections
///
/// Each project has its own database file at {database_dir}/{project_key}.duckdb
pub struct DatabaseFactory {
    database_dir: PathBuf,
    connections: Arc<Mutex<HashMap<String, DbConnection>>>,
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

    /// Get or create a database connection for a specific project
    pub fn get_connection(&self, project_key: &str) -> DomainResult<DbConnection> {
        let mut connections = self.connections.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connections lock: {}", e))
        })?;

        if let Some(conn) = connections.get(project_key) {
            return Ok(conn.clone());
        }

        // Create new connection for this project
        let db_path = self.database_dir.join(format!("{}.duckdb", project_key));
        let db = Database::new(&db_path)?;
        let conn = db.connection();
        connections.insert(project_key.to_string(), conn.clone());

        Ok(conn)
    }

    /// Get the database path for a specific project
    pub fn get_database_path(&self, project_key: &str) -> PathBuf {
        self.database_dir.join(format!("{}.duckdb", project_key))
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
            if path.extension().is_some_and(|ext| ext == "duckdb") {
                if let Some(stem) = path.file_stem() {
                    projects.push(stem.to_string_lossy().to_string());
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

        for (project_key, conn) in connections.iter() {
            checkpoint_connection(conn).map_err(|e| {
                DomainError::Repository(format!(
                    "Failed to checkpoint database for project {}: {}",
                    project_key, e
                ))
            })?;
        }

        Ok(())
    }
}
