use duckdb::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::domain::error::{DomainError, DomainResult};
use super::schema::Schema;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> DomainResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DomainError::Repository(format!("Failed to create database directory: {}", e)))?;
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
}
