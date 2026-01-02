use crate::domain::entities::ChangeHistoryItem;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::ChangeHistoryRepository;
use chrono::{DateTime, Utc};
use duckdb::Connection;
use std::sync::{Arc, Mutex};

pub struct DuckDbChangeHistoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbChangeHistoryRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl ChangeHistoryRepository for DuckDbChangeHistoryRepository {
    fn batch_insert(&self, items: &[ChangeHistoryItem]) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for item in items {
            conn.execute(
                r#"
                INSERT INTO issue_change_history (
                    issue_id, issue_key, history_id,
                    author_account_id, author_display_name,
                    field, field_type,
                    from_value, from_string, to_value, to_string,
                    changed_at, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                duckdb::params![
                    &item.issue_id,
                    &item.issue_key,
                    &item.history_id,
                    &item.author_account_id,
                    &item.author_display_name,
                    &item.field,
                    &item.field_type,
                    &item.from_value,
                    &item.from_string,
                    &item.to_value,
                    &item.to_string,
                    &item.changed_at.to_rfc3339(),
                    &now,
                ],
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to insert change history: {}", e))
            })?;
        }

        Ok(())
    }

    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM issue_change_history WHERE issue_id = ?",
            duckdb::params![issue_id],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to delete change history: {}", e)))?;
        Ok(())
    }

    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<ChangeHistoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
            SELECT issue_id, issue_key, history_id,
                   author_account_id, author_display_name,
                   field, field_type,
                   from_value, from_string, to_value, to_string,
                   CAST(changed_at AS VARCHAR) as changed_at
            FROM issue_change_history
            WHERE issue_key = ?
            ORDER BY changed_at DESC
            "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![issue_key], |row| {
                Ok(ChangeHistoryItem {
                    issue_id: row.get(0)?,
                    issue_key: row.get(1)?,
                    history_id: row.get(2)?,
                    author_account_id: row.get(3)?,
                    author_display_name: row.get(4)?,
                    field: row.get(5)?,
                    field_type: row.get(6)?,
                    from_value: row.get(7)?,
                    from_string: row.get(8)?,
                    to_value: row.get(9)?,
                    to_string: row.get(10)?,
                    changed_at: row
                        .get::<_, String>(11)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut items = Vec::new();
        for item in rows {
            items.push(item.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(items)
    }

    fn find_by_issue_key_and_field(
        &self,
        issue_key: &str,
        field_filter: Option<&str>,
    ) -> DomainResult<Vec<ChangeHistoryItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = if field_filter.is_some() {
            r#"
            SELECT issue_id, issue_key, history_id,
                   author_account_id, author_display_name,
                   field, field_type,
                   from_value, from_string, to_value, to_string,
                   CAST(changed_at AS VARCHAR) as changed_at
            FROM issue_change_history
            WHERE issue_key = ? AND field = ?
            ORDER BY changed_at DESC
            "#
        } else {
            r#"
            SELECT issue_id, issue_key, history_id,
                   author_account_id, author_display_name,
                   field, field_type,
                   from_value, from_string, to_value, to_string,
                   CAST(changed_at AS VARCHAR) as changed_at
            FROM issue_change_history
            WHERE issue_key = ?
            ORDER BY changed_at DESC
            "#
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows: Vec<_> = if let Some(field) = field_filter {
            stmt.query_map(duckdb::params![issue_key, field], |row| {
                Ok(ChangeHistoryItem {
                    issue_id: row.get(0)?,
                    issue_key: row.get(1)?,
                    history_id: row.get(2)?,
                    author_account_id: row.get(3)?,
                    author_display_name: row.get(4)?,
                    field: row.get(5)?,
                    field_type: row.get(6)?,
                    from_value: row.get(7)?,
                    from_string: row.get(8)?,
                    to_value: row.get(9)?,
                    to_string: row.get(10)?,
                    changed_at: row
                        .get::<_, String>(11)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?
            .collect()
        } else {
            stmt.query_map(duckdb::params![issue_key], |row| {
                Ok(ChangeHistoryItem {
                    issue_id: row.get(0)?,
                    issue_key: row.get(1)?,
                    history_id: row.get(2)?,
                    author_account_id: row.get(3)?,
                    author_display_name: row.get(4)?,
                    field: row.get(5)?,
                    field_type: row.get(6)?,
                    from_value: row.get(7)?,
                    from_string: row.get(8)?,
                    to_value: row.get(9)?,
                    to_string: row.get(10)?,
                    changed_at: row
                        .get::<_, String>(11)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?
            .collect()
        };

        let mut items = Vec::new();
        for item in rows {
            items.push(item.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(items)
    }

    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM issue_change_history WHERE issue_key = ?",
                duckdb::params![issue_key],
                |row| row.get(0),
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to count change history: {}", e))
            })?;

        Ok(count as usize)
    }
}
