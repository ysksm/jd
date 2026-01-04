use crate::domain::entities::Issue;
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::{IssueRepository, SearchParams};
use chrono::{DateTime, Utc};
use duckdb::Connection;
use std::sync::{Arc, Mutex};

pub struct DuckDbIssueRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbIssueRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl IssueRepository for DuckDbIssueRepository {
    fn batch_insert(&self, issues: &[Issue]) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for issue in issues {
            let labels_json = issue
                .labels
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let components_json = issue
                .components
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let fix_versions_json = issue
                .fix_versions
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let sprint = issue.sprint.clone();

            let raw_data = issue
                .raw_json
                .as_ref()
                .cloned()
                .unwrap_or_else(|| serde_json::to_string(&issue).unwrap_or_default());

            conn.execute(
                r#"
                INSERT INTO issues (
                    id, project_id, key, summary, description,
                    status, priority, assignee, reporter,
                    issue_type, resolution, labels, components, fix_versions, sprint, parent_key,
                    created_date, updated_date, raw_data, synced_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (id) DO UPDATE SET
                    project_id = excluded.project_id,
                    key = excluded.key,
                    summary = excluded.summary,
                    description = excluded.description,
                    status = excluded.status,
                    priority = excluded.priority,
                    assignee = excluded.assignee,
                    reporter = excluded.reporter,
                    issue_type = excluded.issue_type,
                    resolution = excluded.resolution,
                    labels = excluded.labels,
                    components = excluded.components,
                    fix_versions = excluded.fix_versions,
                    sprint = excluded.sprint,
                    parent_key = excluded.parent_key,
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
                    &issue.issue_type,
                    &issue.resolution,
                    &labels_json,
                    &components_json,
                    &fix_versions_json,
                    &sprint,
                    &issue.parent_key,
                    &issue.created_date.map(|d| d.to_rfc3339()),
                    &issue.updated_date.map(|d| d.to_rfc3339()),
                    &raw_data,
                    &now,
                    &now,
                ],
            )
            .map_err(|e| DomainError::Repository(format!("Failed to insert issue: {}", e)))?;
        }

        Ok(())
    }

    fn find_by_project(&self, project_id: &str) -> DomainResult<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, project_id, key, summary, description,
                   status, priority, assignee, reporter,
                   issue_type, resolution, labels, components, fix_versions, sprint, parent_key,
                   created_date, updated_date, raw_data
            FROM issues
            WHERE project_id = ?
            "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                let labels: Option<Vec<String>> = row
                    .get::<_, Option<String>>(11)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let components: Option<Vec<String>> = row
                    .get::<_, Option<String>>(12)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let fix_versions: Option<Vec<String>> = row
                    .get::<_, Option<String>>(13)?
                    .and_then(|s| serde_json::from_str(&s).ok());

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
                    issue_type: row.get(9)?,
                    resolution: row.get(10)?,
                    labels,
                    components,
                    fix_versions,
                    sprint: row.get(14)?,
                    parent_key: row.get(15)?,
                    created_date: row.get::<_, Option<String>>(16)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    updated_date: row.get::<_, Option<String>>(17)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    raw_json: row.get(18)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut issues = Vec::new();
        for issue in rows {
            issues.push(issue.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(issues)
    }

    fn count_by_project(&self, project_id: &str) -> DomainResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM issues WHERE project_id = ?",
                duckdb::params![project_id],
                |row| row.get(0),
            )
            .map_err(|e| DomainError::Repository(format!("Failed to count issues: {}", e)))?;

        Ok(count as usize)
    }

    fn search(&self, params: &SearchParams) -> DomainResult<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            r#"
            SELECT i.id, i.project_id, i.key, i.summary, i.description,
                   i.status, i.priority, i.assignee, i.reporter,
                   i.issue_type, i.resolution, i.labels, i.components, i.fix_versions, i.sprint, i.parent_key,
                   i.created_date, i.updated_date
            FROM issues i
            LEFT JOIN projects p ON i.project_id = p.id
            WHERE (i.is_deleted IS NULL OR i.is_deleted = false)
            "#,
        );

        let mut conditions = Vec::new();
        let mut sql_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(query) = &params.query {
            conditions.push("(i.summary LIKE ? OR i.description LIKE ?)");
            let search_pattern = format!("%{}%", query);
            sql_params.push(Box::new(search_pattern.clone()));
            sql_params.push(Box::new(search_pattern));
        }

        if let Some(project_key) = &params.project_key {
            // Filter by issue key prefix (e.g., "PROJ-%" for project key "PROJ")
            conditions.push("i.key LIKE ?");
            let project_pattern = format!("{}-%", project_key);
            sql_params.push(Box::new(project_pattern));
        }

        if let Some(status) = &params.status {
            conditions.push("i.status = ?");
            sql_params.push(Box::new(status.clone()));
        }

        if let Some(assignee) = &params.assignee {
            conditions.push("i.assignee LIKE ?");
            let assignee_pattern = format!("%{}%", assignee);
            sql_params.push(Box::new(assignee_pattern));
        }

        if let Some(issue_type) = &params.issue_type {
            conditions.push("i.issue_type = ?");
            sql_params.push(Box::new(issue_type.clone()));
        }

        if let Some(priority) = &params.priority {
            conditions.push("i.priority = ?");
            sql_params.push(Box::new(priority.clone()));
        }

        for condition in conditions {
            sql.push_str(" AND ");
            sql.push_str(condition);
        }

        sql.push_str(" ORDER BY i.created_date DESC");

        if let Some(limit) = params.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let param_refs: Vec<&dyn duckdb::ToSql> = sql_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let labels: Option<Vec<String>> = row
                    .get::<_, Option<String>>(11)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let components: Option<Vec<String>> = row
                    .get::<_, Option<String>>(12)?
                    .and_then(|s| serde_json::from_str(&s).ok());
                let fix_versions: Option<Vec<String>> = row
                    .get::<_, Option<String>>(13)?
                    .and_then(|s| serde_json::from_str(&s).ok());

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
                    issue_type: row.get(9)?,
                    resolution: row.get(10)?,
                    labels,
                    components,
                    fix_versions,
                    sprint: row.get(14)?,
                    parent_key: row.get(15)?,
                    created_date: row.get::<_, Option<String>>(16)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    updated_date: row.get::<_, Option<String>>(17)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    raw_json: None,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut issues = Vec::new();
        for issue in rows {
            issues.push(issue.map_err(|e| DomainError::Repository(e.to_string()))?);
        }

        Ok(issues)
    }

    fn mark_deleted_not_in_keys(&self, project_id: &str, keys: &[String]) -> DomainResult<usize> {
        let conn = self.conn.lock().unwrap();

        if keys.is_empty() {
            // If no keys provided, mark all issues for this project as deleted
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM issues WHERE project_id = ? AND (is_deleted IS NULL OR is_deleted = false)",
                    duckdb::params![project_id],
                    |row| row.get(0),
                )
                .map_err(|e| DomainError::Repository(format!("Failed to count issues: {}", e)))?;

            conn.execute(
                "UPDATE issues SET is_deleted = true WHERE project_id = ?",
                duckdb::params![project_id],
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to mark issues as deleted: {}", e))
            })?;

            return Ok(count as usize);
        }

        // Build a list of placeholders for the IN clause
        let placeholders: Vec<String> = keys.iter().map(|_| "?".to_string()).collect();
        let in_clause = placeholders.join(", ");

        // First, restore any previously deleted issues that are now in the keys list
        let restore_sql = format!(
            "UPDATE issues SET is_deleted = false WHERE project_id = ? AND key IN ({}) AND is_deleted = true",
            in_clause
        );

        let mut restore_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        restore_params.push(Box::new(project_id.to_string()));
        for key in keys {
            restore_params.push(Box::new(key.clone()));
        }

        let restore_refs: Vec<&dyn duckdb::ToSql> =
            restore_params.iter().map(|p| p.as_ref()).collect();

        conn.execute(&restore_sql, restore_refs.as_slice())
            .map_err(|e| DomainError::Repository(format!("Failed to restore issues: {}", e)))?;

        // Count issues to be marked as deleted
        let count_sql = format!(
            "SELECT COUNT(*) FROM issues WHERE project_id = ? AND key NOT IN ({}) AND (is_deleted IS NULL OR is_deleted = false)",
            in_clause
        );

        let mut count_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        count_params.push(Box::new(project_id.to_string()));
        for key in keys {
            count_params.push(Box::new(key.clone()));
        }

        let count_refs: Vec<&dyn duckdb::ToSql> = count_params.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&count_sql, count_refs.as_slice(), |row| row.get(0))
            .map_err(|e| DomainError::Repository(format!("Failed to count issues: {}", e)))?;

        if count > 0 {
            // Mark issues not in the list as deleted
            let update_sql = format!(
                "UPDATE issues SET is_deleted = true WHERE project_id = ? AND key NOT IN ({})",
                in_clause
            );

            let mut update_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
            update_params.push(Box::new(project_id.to_string()));
            for key in keys {
                update_params.push(Box::new(key.clone()));
            }

            let update_refs: Vec<&dyn duckdb::ToSql> =
                update_params.iter().map(|p| p.as_ref()).collect();

            conn.execute(&update_sql, update_refs.as_slice())
                .map_err(|e| {
                    DomainError::Repository(format!("Failed to mark issues as deleted: {}", e))
                })?;
        }

        Ok(count as usize)
    }
}
