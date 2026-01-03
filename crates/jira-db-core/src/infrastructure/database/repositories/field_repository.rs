use crate::domain::entities::JiraField;
use crate::domain::error::{DomainError, DomainResult};
use chrono::Utc;
use duckdb::Connection;
use std::sync::{Arc, Mutex};

pub struct DuckDbFieldRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbFieldRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Upsert JIRA fields metadata
    pub fn upsert_fields(&self, fields: &[JiraField]) -> DomainResult<usize> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        for field in fields {
            conn.execute(
                r#"
                INSERT INTO jira_fields (
                    id, key, name, custom, searchable, navigable, orderable,
                    schema_type, schema_items, schema_system, schema_custom, schema_custom_id,
                    created_at, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    key = excluded.key,
                    name = excluded.name,
                    custom = excluded.custom,
                    searchable = excluded.searchable,
                    navigable = excluded.navigable,
                    orderable = excluded.orderable,
                    schema_type = excluded.schema_type,
                    schema_items = excluded.schema_items,
                    schema_system = excluded.schema_system,
                    schema_custom = excluded.schema_custom,
                    schema_custom_id = excluded.schema_custom_id,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    &field.id,
                    &field.key,
                    &field.name,
                    field.custom,
                    field.searchable,
                    field.navigable,
                    field.orderable,
                    &field.schema_type,
                    &field.schema_items,
                    &field.schema_system,
                    &field.schema_custom,
                    field.schema_custom_id,
                    &now,
                    &now,
                ],
            )
            .map_err(|e| DomainError::Repository(format!("Failed to upsert field: {}", e)))?;
            count += 1;
        }

        Ok(count)
    }

    /// Find all JIRA fields
    pub fn find_all(&self) -> DomainResult<Vec<JiraField>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, key, name, custom, searchable, navigable, orderable,
                       schema_type, schema_items, schema_system, schema_custom, schema_custom_id
                FROM jira_fields
                ORDER BY custom, name
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(JiraField {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                    custom: row.get(3)?,
                    searchable: row.get(4)?,
                    navigable: row.get(5)?,
                    orderable: row.get(6)?,
                    schema_type: row.get(7)?,
                    schema_items: row.get(8)?,
                    schema_system: row.get(9)?,
                    schema_custom: row.get(10)?,
                    schema_custom_id: row.get(11)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut fields = Vec::new();
        for field in rows {
            fields.push(field.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(fields)
    }

    /// Find navigable fields (these are typically useful for display)
    pub fn find_navigable(&self) -> DomainResult<Vec<JiraField>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, key, name, custom, searchable, navigable, orderable,
                       schema_type, schema_items, schema_system, schema_custom, schema_custom_id
                FROM jira_fields
                WHERE navigable = true
                ORDER BY custom, name
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(JiraField {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                    custom: row.get(3)?,
                    searchable: row.get(4)?,
                    navigable: row.get(5)?,
                    orderable: row.get(6)?,
                    schema_type: row.get(7)?,
                    schema_items: row.get(8)?,
                    schema_system: row.get(9)?,
                    schema_custom: row.get(10)?,
                    schema_custom_id: row.get(11)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut fields = Vec::new();
        for field in rows {
            fields.push(field.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(fields)
    }

    /// Find field by ID
    pub fn find_by_id(&self, id: &str) -> DomainResult<Option<JiraField>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, key, name, custom, searchable, navigable, orderable,
                       schema_type, schema_items, schema_system, schema_custom, schema_custom_id
                FROM jira_fields
                WHERE id = ?
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let mut rows = stmt
            .query_map([id], |row| {
                Ok(JiraField {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                    custom: row.get(3)?,
                    searchable: row.get(4)?,
                    navigable: row.get(5)?,
                    orderable: row.get(6)?,
                    schema_type: row.get(7)?,
                    schema_items: row.get(8)?,
                    schema_system: row.get(9)?,
                    schema_custom: row.get(10)?,
                    schema_custom_id: row.get(11)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        match rows.next() {
            Some(Ok(field)) => Ok(Some(field)),
            Some(Err(e)) => Err(DomainError::Repository(e.to_string())),
            None => Ok(None),
        }
    }

    /// Get count of fields
    pub fn count(&self) -> DomainResult<i64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM jira_fields")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let count: i64 = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| DomainError::Repository(format!("Failed to count fields: {}", e)))?;

        Ok(count)
    }

    /// Delete all fields
    pub fn delete_all(&self) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM jira_fields", [])
            .map_err(|e| DomainError::Repository(format!("Failed to delete fields: {}", e)))?;
        Ok(())
    }
}
