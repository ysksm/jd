use chrono::{DateTime, Utc};
use duckdb::Connection;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

use crate::domain::entities::IssueSnapshot;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::IssueSnapshotRepository;

pub struct DuckDbIssueSnapshotRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbIssueSnapshotRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn parse_json_array(s: Option<String>) -> Option<Vec<String>> {
        s.and_then(|v| serde_json::from_str(&v).ok())
    }

    fn serialize_json_array(arr: &Option<Vec<String>>) -> Option<String> {
        arr.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
    }

    fn parse_json_value(s: Option<String>) -> Option<JsonValue> {
        s.and_then(|v| serde_json::from_str(&v).ok())
    }
}

impl IssueSnapshotRepository for DuckDbIssueSnapshotRepository {
    fn batch_insert(&self, snapshots: &[IssueSnapshot]) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;

        for snapshot in snapshots {
            let labels_json = Self::serialize_json_array(&snapshot.labels);
            let components_json = Self::serialize_json_array(&snapshot.components);
            let fix_versions_json = Self::serialize_json_array(&snapshot.fix_versions);
            let valid_to_str = snapshot.valid_to.map(|dt| dt.to_rfc3339());
            let raw_data_str = snapshot
                .raw_data
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let updated_date_str = snapshot.updated_date.map(|dt| dt.to_rfc3339());

            conn.execute(
                r#"
                INSERT INTO issue_snapshots (
                    issue_id, issue_key, project_id, version,
                    valid_from, valid_to,
                    summary, description, status, priority,
                    assignee, reporter, issue_type, resolution,
                    labels, components, fix_versions, sprint, parent_key,
                    raw_data, updated_date, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (issue_id, version) DO UPDATE SET
                    valid_to = EXCLUDED.valid_to,
                    summary = EXCLUDED.summary,
                    description = EXCLUDED.description,
                    status = EXCLUDED.status,
                    priority = EXCLUDED.priority,
                    assignee = EXCLUDED.assignee,
                    reporter = EXCLUDED.reporter,
                    issue_type = EXCLUDED.issue_type,
                    resolution = EXCLUDED.resolution,
                    labels = EXCLUDED.labels,
                    components = EXCLUDED.components,
                    fix_versions = EXCLUDED.fix_versions,
                    sprint = EXCLUDED.sprint,
                    parent_key = EXCLUDED.parent_key,
                    raw_data = EXCLUDED.raw_data,
                    updated_date = EXCLUDED.updated_date
                "#,
                duckdb::params![
                    &snapshot.issue_id,
                    &snapshot.issue_key,
                    &snapshot.project_id,
                    &snapshot.version,
                    &snapshot.valid_from.to_rfc3339(),
                    &valid_to_str,
                    &snapshot.summary,
                    &snapshot.description,
                    &snapshot.status,
                    &snapshot.priority,
                    &snapshot.assignee,
                    &snapshot.reporter,
                    &snapshot.issue_type,
                    &snapshot.resolution,
                    &labels_json,
                    &components_json,
                    &fix_versions_json,
                    &snapshot.sprint,
                    &snapshot.parent_key,
                    &raw_data_str,
                    &updated_date_str,
                    &snapshot.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to insert issue snapshot: {}", e))
            })?;
        }

        Ok(())
    }

    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        conn.execute(
            "DELETE FROM issue_snapshots WHERE issue_id = ?",
            duckdb::params![issue_id],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to delete issue snapshots: {}", e)))?;
        Ok(())
    }

    fn delete_by_project_id(&self, project_id: &str) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        conn.execute(
            "DELETE FROM issue_snapshots WHERE project_id = ?",
            duckdb::params![project_id],
        )
        .map_err(|e| {
            DomainError::Repository(format!("Failed to delete project snapshots: {}", e))
        })?;
        Ok(())
    }

    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<IssueSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT issue_id, issue_key, project_id, version,
                       CAST(valid_from AS VARCHAR), CAST(valid_to AS VARCHAR),
                       summary, description, status, priority,
                       assignee, reporter, issue_type, resolution,
                       labels, components, fix_versions, sprint, parent_key,
                       raw_data, CAST(updated_date AS VARCHAR), CAST(created_at AS VARCHAR)
                FROM issue_snapshots
                WHERE issue_key = ?
                ORDER BY version ASC
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![issue_key], |row| {
                Ok(IssueSnapshot {
                    issue_id: row.get(0)?,
                    issue_key: row.get(1)?,
                    project_id: row.get(2)?,
                    version: row.get(3)?,
                    valid_from: row
                        .get::<_, String>(4)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    valid_to: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    summary: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    priority: row.get(9)?,
                    assignee: row.get(10)?,
                    reporter: row.get(11)?,
                    issue_type: row.get(12)?,
                    resolution: row.get(13)?,
                    labels: Self::parse_json_array(row.get(14)?),
                    components: Self::parse_json_array(row.get(15)?),
                    fix_versions: Self::parse_json_array(row.get(16)?),
                    sprint: row.get(17)?,
                    parent_key: row.get(18)?,
                    raw_data: Self::parse_json_value(row.get(19)?),
                    updated_date: row
                        .get::<_, Option<String>>(20)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    created_at: row
                        .get::<_, String>(21)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(snapshots)
    }

    fn find_by_issue_key_and_version(
        &self,
        issue_key: &str,
        version: i32,
    ) -> DomainResult<Option<IssueSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT issue_id, issue_key, project_id, version,
                       CAST(valid_from AS VARCHAR), CAST(valid_to AS VARCHAR),
                       summary, description, status, priority,
                       assignee, reporter, issue_type, resolution,
                       labels, components, fix_versions, sprint, parent_key,
                       raw_data, CAST(updated_date AS VARCHAR), CAST(created_at AS VARCHAR)
                FROM issue_snapshots
                WHERE issue_key = ? AND version = ?
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let result = stmt.query_row(duckdb::params![issue_key, version], |row| {
            Ok(IssueSnapshot {
                issue_id: row.get(0)?,
                issue_key: row.get(1)?,
                project_id: row.get(2)?,
                version: row.get(3)?,
                valid_from: row
                    .get::<_, String>(4)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
                valid_to: row
                    .get::<_, Option<String>>(5)?
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                summary: row.get(6)?,
                description: row.get(7)?,
                status: row.get(8)?,
                priority: row.get(9)?,
                assignee: row.get(10)?,
                reporter: row.get(11)?,
                issue_type: row.get(12)?,
                resolution: row.get(13)?,
                labels: Self::parse_json_array(row.get(14)?),
                components: Self::parse_json_array(row.get(15)?),
                fix_versions: Self::parse_json_array(row.get(16)?),
                sprint: row.get(17)?,
                parent_key: row.get(18)?,
                raw_data: Self::parse_json_value(row.get(19)?),
                updated_date: row
                    .get::<_, Option<String>>(20)?
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                created_at: row
                    .get::<_, String>(21)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Repository(format!(
                "Failed to find snapshot: {}",
                e
            ))),
        }
    }

    fn find_current_by_issue_key(&self, issue_key: &str) -> DomainResult<Option<IssueSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT issue_id, issue_key, project_id, version,
                       CAST(valid_from AS VARCHAR), CAST(valid_to AS VARCHAR),
                       summary, description, status, priority,
                       assignee, reporter, issue_type, resolution,
                       labels, components, fix_versions, sprint, parent_key,
                       raw_data, CAST(updated_date AS VARCHAR), CAST(created_at AS VARCHAR)
                FROM issue_snapshots
                WHERE issue_key = ? AND valid_to IS NULL
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let result = stmt.query_row(duckdb::params![issue_key], |row| {
            Ok(IssueSnapshot {
                issue_id: row.get(0)?,
                issue_key: row.get(1)?,
                project_id: row.get(2)?,
                version: row.get(3)?,
                valid_from: row
                    .get::<_, String>(4)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
                valid_to: row
                    .get::<_, Option<String>>(5)?
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                summary: row.get(6)?,
                description: row.get(7)?,
                status: row.get(8)?,
                priority: row.get(9)?,
                assignee: row.get(10)?,
                reporter: row.get(11)?,
                issue_type: row.get(12)?,
                resolution: row.get(13)?,
                labels: Self::parse_json_array(row.get(14)?),
                components: Self::parse_json_array(row.get(15)?),
                fix_versions: Self::parse_json_array(row.get(16)?),
                sprint: row.get(17)?,
                parent_key: row.get(18)?,
                raw_data: Self::parse_json_value(row.get(19)?),
                updated_date: row
                    .get::<_, Option<String>>(20)?
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                created_at: row
                    .get::<_, String>(21)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainError::Repository(format!(
                "Failed to find current snapshot: {}",
                e
            ))),
        }
    }

    fn find_by_project_id(&self, project_id: &str) -> DomainResult<Vec<IssueSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT issue_id, issue_key, project_id, version,
                       CAST(valid_from AS VARCHAR), CAST(valid_to AS VARCHAR),
                       summary, description, status, priority,
                       assignee, reporter, issue_type, resolution,
                       labels, components, fix_versions, sprint, parent_key,
                       raw_data, CAST(updated_date AS VARCHAR), CAST(created_at AS VARCHAR)
                FROM issue_snapshots
                WHERE project_id = ?
                ORDER BY issue_key, version ASC
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(IssueSnapshot {
                    issue_id: row.get(0)?,
                    issue_key: row.get(1)?,
                    project_id: row.get(2)?,
                    version: row.get(3)?,
                    valid_from: row
                        .get::<_, String>(4)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    valid_to: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    summary: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    priority: row.get(9)?,
                    assignee: row.get(10)?,
                    reporter: row.get(11)?,
                    issue_type: row.get(12)?,
                    resolution: row.get(13)?,
                    labels: Self::parse_json_array(row.get(14)?),
                    components: Self::parse_json_array(row.get(15)?),
                    fix_versions: Self::parse_json_array(row.get(16)?),
                    sprint: row.get(17)?,
                    parent_key: row.get(18)?,
                    raw_data: Self::parse_json_value(row.get(19)?),
                    updated_date: row
                        .get::<_, Option<String>>(20)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    created_at: row
                        .get::<_, String>(21)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(snapshots)
    }

    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM issue_snapshots WHERE issue_key = ?",
                duckdb::params![issue_key],
                |row| row.get(0),
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to count issue snapshots: {}", e))
            })?;

        Ok(count as usize)
    }

    fn count_by_project_id(&self, project_id: &str) -> DomainResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM issue_snapshots WHERE project_id = ?",
                duckdb::params![project_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to count project snapshots: {}", e))
            })?;

        Ok(count as usize)
    }

    fn bulk_insert(&self, snapshots: &[IssueSnapshot]) -> DomainResult<()> {
        if snapshots.is_empty() {
            return Ok(());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;

        // Use prepared statement with batched execution for better performance
        // DuckDB doesn't have an Appender API in the Rust bindings, so we use
        // a prepared statement with explicit transaction for bulk inserts
        let mut stmt = conn
            .prepare(
                r#"
                INSERT INTO issue_snapshots (
                    issue_id, issue_key, project_id, version,
                    valid_from, valid_to,
                    summary, description, status, priority,
                    assignee, reporter, issue_type, resolution,
                    labels, components, fix_versions, sprint, parent_key,
                    raw_data, updated_date, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (issue_id, version) DO UPDATE SET
                    valid_to = EXCLUDED.valid_to,
                    summary = EXCLUDED.summary,
                    description = EXCLUDED.description,
                    status = EXCLUDED.status,
                    priority = EXCLUDED.priority,
                    assignee = EXCLUDED.assignee,
                    reporter = EXCLUDED.reporter,
                    issue_type = EXCLUDED.issue_type,
                    resolution = EXCLUDED.resolution,
                    labels = EXCLUDED.labels,
                    components = EXCLUDED.components,
                    fix_versions = EXCLUDED.fix_versions,
                    sprint = EXCLUDED.sprint,
                    parent_key = EXCLUDED.parent_key,
                    raw_data = EXCLUDED.raw_data,
                    updated_date = EXCLUDED.updated_date
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare statement: {}", e)))?;

        for snapshot in snapshots {
            let labels_json = Self::serialize_json_array(&snapshot.labels);
            let components_json = Self::serialize_json_array(&snapshot.components);
            let fix_versions_json = Self::serialize_json_array(&snapshot.fix_versions);
            let valid_to_str = snapshot.valid_to.map(|dt| dt.to_rfc3339());
            let raw_data_str = snapshot
                .raw_data
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());
            let updated_date_str = snapshot.updated_date.map(|dt| dt.to_rfc3339());

            stmt.execute(duckdb::params![
                &snapshot.issue_id,
                &snapshot.issue_key,
                &snapshot.project_id,
                &snapshot.version,
                &snapshot.valid_from.to_rfc3339(),
                &valid_to_str,
                &snapshot.summary,
                &snapshot.description,
                &snapshot.status,
                &snapshot.priority,
                &snapshot.assignee,
                &snapshot.reporter,
                &snapshot.issue_type,
                &snapshot.resolution,
                &labels_json,
                &components_json,
                &fix_versions_json,
                &snapshot.sprint,
                &snapshot.parent_key,
                &raw_data_str,
                &updated_date_str,
                &snapshot.created_at.to_rfc3339(),
            ])
            .map_err(|e| DomainError::Repository(format!("Failed to insert snapshot: {}", e)))?;
        }

        Ok(())
    }

    fn begin_transaction(&self) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| DomainError::Repository(format!("Failed to begin transaction: {}", e)))?;
        Ok(())
    }

    fn commit_transaction(&self) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        conn.execute("COMMIT", [])
            .map_err(|e| DomainError::Repository(format!("Failed to commit transaction: {}", e)))?;
        Ok(())
    }

    fn rollback_transaction(&self) -> DomainResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;
        conn.execute("ROLLBACK", []).map_err(|e| {
            DomainError::Repository(format!("Failed to rollback transaction: {}", e))
        })?;
        Ok(())
    }
}
