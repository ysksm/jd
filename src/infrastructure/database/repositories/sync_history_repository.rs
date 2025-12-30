use chrono::{DateTime, Utc};
use duckdb::Connection;
use std::sync::{Arc, Mutex};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::SyncHistoryRepository;

pub struct DuckDbSyncHistoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbSyncHistoryRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl SyncHistoryRepository for DuckDbSyncHistoryRepository {
    fn insert(
        &self,
        project_id: &str,
        sync_type: &str,
        started_at: DateTime<Utc>,
    ) -> DomainResult<i64> {
        let conn = self.conn.lock().unwrap();

        let id: i64 = conn
            .query_row(
                r#"
            INSERT INTO sync_history (project_id, sync_type, started_at, status)
            VALUES (?, ?, ?, 'running')
            RETURNING id
            "#,
                duckdb::params![project_id, sync_type, started_at.to_rfc3339()],
                |row| row.get(0),
            )
            .map_err(|e| DomainError::Repository(format!("Failed to insert sync history: {}", e)))?;

        Ok(id)
    }

    fn update_completed(
        &self,
        id: i64,
        items_synced: usize,
        completed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE sync_history
            SET status = 'completed', completed_at = ?, items_synced = ?
            WHERE id = ?
            "#,
            duckdb::params![completed_at.to_rfc3339(), items_synced as i64, id],
        ).map_err(|e| DomainError::Repository(format!("Failed to update sync history: {}", e)))?;
        Ok(())
    }

    fn update_failed(
        &self,
        id: i64,
        error_message: &str,
        completed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE sync_history
            SET status = 'failed', completed_at = ?, error_message = ?
            WHERE id = ?
            "#,
            duckdb::params![completed_at.to_rfc3339(), error_message, id],
        ).map_err(|e| DomainError::Repository(format!("Failed to update sync history: {}", e)))?;
        Ok(())
    }

    fn find_latest_by_project(&self, project_id: &str) -> DomainResult<Option<(DateTime<Utc>, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
            SELECT completed_at, status
            FROM sync_history
            WHERE project_id = ? AND status = 'completed'
            ORDER BY completed_at DESC
            LIMIT 1
            "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let mut rows = stmt
            .query(duckdb::params![project_id])
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        if let Some(row) = rows.next().map_err(|e| DomainError::Repository(e.to_string()))? {
            let completed_at_str: String =
                row.get(0).map_err(|e| DomainError::Repository(e.to_string()))?;
            let status: String =
                row.get(1).map_err(|e| DomainError::Repository(e.to_string()))?;

            if let Ok(dt) = DateTime::parse_from_rfc3339(&completed_at_str) {
                Ok(Some((dt.with_timezone(&Utc), status)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
