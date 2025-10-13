use crate::error::Result;
use crate::jira::models::{Issue, Project};
use chrono::{DateTime, Utc};
use duckdb::Connection;
use std::sync::{Arc, Mutex};

pub struct ProjectRepository {
    conn: Arc<Mutex<Connection>>,
}

impl ProjectRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, project: &Project) -> Result<()> {
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
                &serde_json::to_string(&project)?,
                &now,
                &now,
                &now,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_key(&self, key: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, name, description FROM projects WHERE key = ?",
        )?;

        let mut rows = stmt.query(duckdb::params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Project {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn find_all(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, key, name, description FROM projects")?;

        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
            })
        })?;

        let mut projects = Vec::new();
        for project in rows {
            projects.push(project?);
        }

        Ok(projects)
    }
}

pub struct IssueRepository {
    conn: Arc<Mutex<Connection>>,
}

impl IssueRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn batch_insert(&self, issues: &[Issue]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for issue in issues {
            conn.execute(
                r#"
                INSERT INTO issues (
                    id, project_id, key, summary, description,
                    status, priority, assignee, reporter,
                    created_date, updated_date, raw_data, synced_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    project_id = excluded.project_id,
                    key = excluded.key,
                    summary = excluded.summary,
                    description = excluded.description,
                    status = excluded.status,
                    priority = excluded.priority,
                    assignee = excluded.assignee,
                    reporter = excluded.reporter,
                    created_date = excluded.created_date,
                    updated_date = excluded.updated_date,
                    raw_data = excluded.raw_data,
                    synced_at = ?
                "#,
                duckdb::params![
                    &issue.id,
                    &issue.project_id,
                    &issue.key,
                    &issue.summary,
                    &issue.description,
                    &issue.status,
                    &issue.priority,
                    &issue.assignee,
                    &issue.reporter,
                    &issue.created_date.map(|d| d.to_rfc3339()),
                    &issue.updated_date.map(|d| d.to_rfc3339()),
                    &serde_json::to_string(&issue)?,
                    &now,
                    &now,
                ],
            )?;
        }

        Ok(())
    }

    pub fn find_by_project(&self, project_id: &str) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, key, summary, description,
                   status, priority, assignee, reporter,
                   created_date, updated_date
            FROM issues
            WHERE project_id = ?
            "#,
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(Issue {
                id: row.get(0)?,
                project_id: row.get(1)?,
                key: row.get(2)?,
                summary: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                assignee: row.get(7)?,
                reporter: row.get(8)?,
                created_date: row.get::<_, Option<String>>(9)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                updated_date: row.get::<_, Option<String>>(10)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            })
        })?;

        let mut issues = Vec::new();
        for issue in rows {
            issues.push(issue?);
        }

        Ok(issues)
    }

    pub fn count_by_project(&self, project_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE project_id = ?",
            duckdb::params![project_id],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }
}

pub struct SyncHistoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SyncHistoryRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(
        &self,
        project_id: &str,
        sync_type: &str,
        started_at: DateTime<Utc>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        // Use RETURNING to get the inserted ID
        let id: i64 = conn.query_row(
            r#"
            INSERT INTO sync_history (project_id, sync_type, started_at, status)
            VALUES (?, ?, ?, 'running')
            RETURNING id
            "#,
            duckdb::params![project_id, sync_type, started_at.to_rfc3339()],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn update_completed(
        &self,
        id: i64,
        items_synced: usize,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE sync_history
            SET status = 'completed', completed_at = ?, items_synced = ?
            WHERE id = ?
            "#,
            duckdb::params![completed_at.to_rfc3339(), items_synced as i64, id],
        )?;
        Ok(())
    }

    pub fn update_failed(&self, id: i64, error_message: &str, completed_at: DateTime<Utc>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE sync_history
            SET status = 'failed', completed_at = ?, error_message = ?
            WHERE id = ?
            "#,
            duckdb::params![completed_at.to_rfc3339(), error_message, id],
        )?;
        Ok(())
    }

    pub fn find_latest_by_project(&self, project_id: &str) -> Result<Option<(DateTime<Utc>, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT completed_at, status
            FROM sync_history
            WHERE project_id = ? AND status = 'completed'
            ORDER BY completed_at DESC
            LIMIT 1
            "#,
        )?;

        let mut rows = stmt.query(duckdb::params![project_id])?;

        if let Some(row) = rows.next()? {
            let completed_at_str: String = row.get(0)?;
            let status: String = row.get(1)?;

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
