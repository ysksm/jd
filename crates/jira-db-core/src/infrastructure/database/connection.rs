use super::schema::Schema;
use crate::domain::error::{DomainError, DomainResult};
use duckdb::Connection;
use std::path::Path;
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
