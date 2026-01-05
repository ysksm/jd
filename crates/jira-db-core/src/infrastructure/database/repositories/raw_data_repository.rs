//! Raw data repository for storing JIRA API response JSON

use std::sync::Arc;

use chrono::Utc;
use duckdb::params;

use crate::domain::error::{DomainError, DomainResult};
use crate::infrastructure::database::DbConnection;

/// Repository for storing raw JIRA API responses
pub struct RawDataRepository {
    conn: DbConnection,
}

impl RawDataRepository {
    pub fn new(conn: DbConnection) -> Self {
        Self { conn }
    }

    /// Insert or update raw issue data
    pub fn upsert_issue_raw_data(
        &self,
        id: &str,
        issue_key: &str,
        project_id: &str,
        raw_data: &str,
    ) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO issue_raw_data (id, issue_key, project_id, raw_data, synced_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                issue_key = EXCLUDED.issue_key,
                project_id = EXCLUDED.project_id,
                raw_data = EXCLUDED.raw_data,
                synced_at = EXCLUDED.synced_at
            "#,
            params![id, issue_key, project_id, raw_data, now],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to upsert issue raw data: {}", e)))?;

        Ok(())
    }

    /// Batch insert raw issue data
    pub fn batch_upsert_issue_raw_data(
        &self,
        items: &[(String, String, String, String)], // (id, issue_key, project_id, raw_data)
    ) -> DomainResult<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let now = Utc::now().to_rfc3339();
        let mut count = 0;

        for (id, issue_key, project_id, raw_data) in items {
            conn.execute(
                r#"
                INSERT INTO issue_raw_data (id, issue_key, project_id, raw_data, synced_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    issue_key = EXCLUDED.issue_key,
                    project_id = EXCLUDED.project_id,
                    raw_data = EXCLUDED.raw_data,
                    synced_at = EXCLUDED.synced_at
                "#,
                params![id, issue_key, project_id, raw_data, &now],
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to upsert issue raw data: {}", e))
            })?;
            count += 1;
        }

        Ok(count)
    }

    /// Get raw data for an issue
    pub fn get_issue_raw_data(&self, issue_key: &str) -> DomainResult<Option<String>> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT raw_data FROM issue_raw_data WHERE issue_key = ?")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare statement: {}", e)))?;

        let result = stmt.query_row([issue_key], |row| row.get::<_, String>(0));

        match result {
            Ok(data) => Ok(Some(data)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Repository(format!(
                "Failed to get issue raw data: {}",
                e
            ))),
        }
    }

    /// Insert or update raw project data
    pub fn upsert_project_raw_data(
        &self,
        id: &str,
        project_key: &str,
        raw_data: &str,
    ) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO project_raw_data (id, project_key, raw_data, synced_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                project_key = EXCLUDED.project_key,
                raw_data = EXCLUDED.raw_data,
                synced_at = EXCLUDED.synced_at
            "#,
            params![id, project_key, raw_data, now],
        )
        .map_err(|e| {
            DomainError::Repository(format!("Failed to upsert project raw data: {}", e))
        })?;

        Ok(())
    }

    /// Get raw data for a project
    pub fn get_project_raw_data(&self, project_key: &str) -> DomainResult<Option<String>> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT raw_data FROM project_raw_data WHERE project_key = ?")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare statement: {}", e)))?;

        let result = stmt.query_row([project_key], |row| row.get::<_, String>(0));

        match result {
            Ok(data) => Ok(Some(data)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Repository(format!(
                "Failed to get project raw data: {}",
                e
            ))),
        }
    }

    /// Count raw issue data entries
    pub fn count_issues(&self) -> DomainResult<usize> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM issue_raw_data", [], |row| row.get(0))
            .map_err(|e| DomainError::Repository(format!("Failed to count raw issues: {}", e)))?;

        Ok(count as usize)
    }
}

/// Wrapper for using RawDataRepository with Arc
pub type SharedRawDataRepository = Arc<RawDataRepository>;
