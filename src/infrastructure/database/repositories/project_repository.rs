use chrono::Utc;
use duckdb::Connection;
use std::sync::{Arc, Mutex};
use crate::domain::entities::Project;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::ProjectRepository;

pub struct DuckDbProjectRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbProjectRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl ProjectRepository for DuckDbProjectRepository {
    fn insert(&self, project: &Project) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO projects (id, key, name, description, raw_data, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                key = excluded.key,
                name = excluded.name,
                description = excluded.description,
                raw_data = excluded.raw_data,
                updated_at = ?
            "#,
            duckdb::params![
                &project.id,
                &project.key,
                &project.name,
                &project.description,
                &serde_json::to_string(&project).map_err(|e| DomainError::Repository(e.to_string()))?,
                &now,
                &now,
                &now,
            ],
        ).map_err(|e| DomainError::Repository(format!("Failed to insert project: {}", e)))?;
        Ok(())
    }

    fn find_by_key(&self, key: &str) -> DomainResult<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key, name, description FROM projects WHERE key = ?")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let mut rows = stmt
            .query(duckdb::params![key])
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        if let Some(row) = rows.next().map_err(|e| DomainError::Repository(e.to_string()))? {
            Ok(Some(Project {
                id: row.get(0).map_err(|e| DomainError::Repository(e.to_string()))?,
                key: row.get(1).map_err(|e| DomainError::Repository(e.to_string()))?,
                name: row.get(2).map_err(|e| DomainError::Repository(e.to_string()))?,
                description: row.get(3).map_err(|e| DomainError::Repository(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    fn find_all(&self) -> DomainResult<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key, name, description FROM projects")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut projects = Vec::new();
        for project in rows {
            projects.push(project.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(projects)
    }
}
