use crate::error::Result;
use crate::jira::models::{ChangeHistoryItem, Component, FixVersion, Issue, IssueType, Label, Priority, Project, Status};
use chrono::{DateTime, Utc};
use duckdb::Connection;
use std::sync::{Arc, Mutex};

/// Search parameters for issues
#[derive(Debug, Default)]
pub struct SearchParams {
    pub query: Option<String>,
    pub project_key: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

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
            // Convert array fields to JSON strings
            let labels_json = issue.labels
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let components_json = issue.components
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            let fix_versions_json = issue.fix_versions
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default());

            // Use raw_json if available, otherwise serialize the issue
            let raw_data = issue.raw_json.as_ref()
                .map(|s| s.clone())
                .unwrap_or_else(|| serde_json::to_string(&issue).unwrap_or_default());

            conn.execute(
                r#"
                INSERT INTO issues (
                    id, project_id, key, summary, description,
                    status, priority, assignee, reporter,
                    issue_type, resolution, labels, components, fix_versions, parent_key,
                    created_date, updated_date, raw_data, synced_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                    &issue.parent_key,
                    &issue.created_date.map(|d| d.to_rfc3339()),
                    &issue.updated_date.map(|d| d.to_rfc3339()),
                    &raw_data,
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
                   issue_type, resolution, labels, components, fix_versions, parent_key,
                   created_date, updated_date
            FROM issues
            WHERE project_id = ?
            "#,
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            // Parse JSON strings back to Vec<String>
            let labels: Option<Vec<String>> = row.get::<_, Option<String>>(11)?
                .and_then(|s| serde_json::from_str(&s).ok());
            let components: Option<Vec<String>> = row.get::<_, Option<String>>(12)?
                .and_then(|s| serde_json::from_str(&s).ok());
            let fix_versions: Option<Vec<String>> = row.get::<_, Option<String>>(13)?
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
                parent_key: row.get(14)?,
                created_date: row.get::<_, Option<String>>(15)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                updated_date: row.get::<_, Option<String>>(16)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                raw_json: None,
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

    /// Search issues with filters
    pub fn search(&self, params: &SearchParams) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();

        // Build SQL query dynamically
        let mut sql = String::from(
            r#"
            SELECT i.id, i.project_id, i.key, i.summary, i.description,
                   i.status, i.priority, i.assignee, i.reporter,
                   i.issue_type, i.resolution, i.labels, i.components, i.fix_versions, i.parent_key,
                   i.created_date, i.updated_date
            FROM issues i
            LEFT JOIN projects p ON i.project_id = p.id
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        let mut sql_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        // Text search in summary and description
        if let Some(query) = &params.query {
            conditions.push("(i.summary LIKE ? OR i.description LIKE ?)");
            let search_pattern = format!("%{}%", query);
            sql_params.push(Box::new(search_pattern.clone()));
            sql_params.push(Box::new(search_pattern));
        }

        // Project key filter
        if let Some(project_key) = &params.project_key {
            conditions.push("p.key = ?");
            sql_params.push(Box::new(project_key.clone()));
        }

        // Status filter
        if let Some(status) = &params.status {
            conditions.push("i.status = ?");
            sql_params.push(Box::new(status.clone()));
        }

        // Assignee filter
        if let Some(assignee) = &params.assignee {
            conditions.push("i.assignee LIKE ?");
            let assignee_pattern = format!("%{}%", assignee);
            sql_params.push(Box::new(assignee_pattern));
        }

        // Add conditions to SQL
        for condition in conditions {
            sql.push_str(" AND ");
            sql.push_str(condition);
        }

        // Order by created date descending
        sql.push_str(" ORDER BY i.created_date DESC");

        // Pagination
        if let Some(limit) = params.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        // Prepare statement
        let mut stmt = conn.prepare(&sql)?;

        // Convert params to references
        let param_refs: Vec<&dyn duckdb::ToSql> = sql_params.iter().map(|p| p.as_ref()).collect();

        // Execute query
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            // Parse JSON strings back to Vec<String>
            let labels: Option<Vec<String>> = row.get::<_, Option<String>>(11)?
                .and_then(|s| serde_json::from_str(&s).ok());
            let components: Option<Vec<String>> = row.get::<_, Option<String>>(12)?
                .and_then(|s| serde_json::from_str(&s).ok());
            let fix_versions: Option<Vec<String>> = row.get::<_, Option<String>>(13)?
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
                parent_key: row.get(14)?,
                created_date: row.get::<_, Option<String>>(15)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                updated_date: row.get::<_, Option<String>>(16)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                raw_json: None,
            })
        })?;

        let mut issues = Vec::new();
        for issue in rows {
            issues.push(issue?);
        }

        Ok(issues)
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

pub struct MetadataRepository {
    conn: Arc<Mutex<Connection>>,
}

impl MetadataRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    // Status operations
    pub fn upsert_statuses(&self, project_id: &str, statuses: &[Status]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for status in statuses {
            conn.execute(
                r#"
                INSERT INTO statuses (project_id, name, description, category, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    description = excluded.description,
                    category = excluded.category,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &status.name,
                    &status.description,
                    &status.category,
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_statuses_by_project(&self, project_id: &str) -> Result<Vec<Status>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, description, category FROM statuses WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(Status {
                name: row.get(0)?,
                description: row.get(1)?,
                category: row.get(2)?,
            })
        })?;

        let mut statuses = Vec::new();
        for status in rows {
            statuses.push(status?);
        }
        Ok(statuses)
    }

    // Priority operations
    pub fn upsert_priorities(&self, project_id: &str, priorities: &[Priority]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for priority in priorities {
            conn.execute(
                r#"
                INSERT INTO priorities (project_id, name, description, icon_url, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    description = excluded.description,
                    icon_url = excluded.icon_url,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &priority.name,
                    &priority.description,
                    &priority.icon_url,
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_priorities_by_project(&self, project_id: &str) -> Result<Vec<Priority>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, description, icon_url FROM priorities WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(Priority {
                name: row.get(0)?,
                description: row.get(1)?,
                icon_url: row.get(2)?,
            })
        })?;

        let mut priorities = Vec::new();
        for priority in rows {
            priorities.push(priority?);
        }
        Ok(priorities)
    }

    // IssueType operations
    pub fn upsert_issue_types(&self, project_id: &str, issue_types: &[IssueType]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for issue_type in issue_types {
            conn.execute(
                r#"
                INSERT INTO issue_types (project_id, name, description, icon_url, subtask, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    description = excluded.description,
                    icon_url = excluded.icon_url,
                    subtask = excluded.subtask,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &issue_type.name,
                    &issue_type.description,
                    &issue_type.icon_url,
                    &issue_type.subtask,
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_issue_types_by_project(&self, project_id: &str) -> Result<Vec<IssueType>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, description, icon_url, subtask FROM issue_types WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(IssueType {
                name: row.get(0)?,
                description: row.get(1)?,
                icon_url: row.get(2)?,
                subtask: row.get(3)?,
            })
        })?;

        let mut issue_types = Vec::new();
        for issue_type in rows {
            issue_types.push(issue_type?);
        }
        Ok(issue_types)
    }

    // Label operations
    pub fn upsert_labels(&self, project_id: &str, labels: &[Label]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for label in labels {
            conn.execute(
                r#"
                INSERT INTO labels (project_id, name, created_at, updated_at)
                VALUES (?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &label.name,
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_labels_by_project(&self, project_id: &str) -> Result<Vec<Label>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM labels WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(Label {
                name: row.get(0)?,
            })
        })?;

        let mut labels = Vec::new();
        for label in rows {
            labels.push(label?);
        }
        Ok(labels)
    }

    // Component operations
    pub fn upsert_components(&self, project_id: &str, components: &[Component]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for component in components {
            conn.execute(
                r#"
                INSERT INTO components (project_id, name, description, lead, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    description = excluded.description,
                    lead = excluded.lead,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &component.name,
                    &component.description,
                    &component.lead,
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_components_by_project(&self, project_id: &str) -> Result<Vec<Component>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, description, lead FROM components WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(Component {
                name: row.get(0)?,
                description: row.get(1)?,
                lead: row.get(2)?,
            })
        })?;

        let mut components = Vec::new();
        for component in rows {
            components.push(component?);
        }
        Ok(components)
    }

    // FixVersion operations
    pub fn upsert_fix_versions(&self, project_id: &str, fix_versions: &[FixVersion]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for version in fix_versions {
            conn.execute(
                r#"
                INSERT INTO fix_versions (project_id, name, description, released, release_date, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (project_id, name) DO UPDATE SET
                    description = excluded.description,
                    released = excluded.released,
                    release_date = excluded.release_date,
                    updated_at = excluded.updated_at
                "#,
                duckdb::params![
                    project_id,
                    &version.name,
                    &version.description,
                    &version.released,
                    &version.release_date.map(|d| d.to_rfc3339()),
                    &now,
                    &now,
                ],
            )?;
        }
        Ok(())
    }

    pub fn find_fix_versions_by_project(&self, project_id: &str) -> Result<Vec<FixVersion>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, description, released, release_date FROM fix_versions WHERE project_id = ? ORDER BY name",
        )?;

        let rows = stmt.query_map(duckdb::params![project_id], |row| {
            Ok(FixVersion {
                name: row.get(0)?,
                description: row.get(1)?,
                released: row.get(2)?,
                release_date: row.get::<_, Option<String>>(3)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            })
        })?;

        let mut fix_versions = Vec::new();
        for version in rows {
            fix_versions.push(version?);
        }
        Ok(fix_versions)
    }
}

pub struct ChangeHistoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl ChangeHistoryRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Insert change history items in batch
    pub fn batch_insert(&self, items: &[ChangeHistoryItem]) -> Result<()> {
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
            )?;
        }

        Ok(())
    }

    /// Delete all change history for an issue (used before re-syncing)
    pub fn delete_by_issue_id(&self, issue_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM issue_change_history WHERE issue_id = ?",
            duckdb::params![issue_id],
        )?;
        Ok(())
    }

    /// Find change history by issue key
    pub fn find_by_issue_key(&self, issue_key: &str) -> Result<Vec<ChangeHistoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
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
        )?;

        let rows = stmt.query_map(duckdb::params![issue_key], |row| {
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
                changed_at: row.get::<_, String>(11)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut items = Vec::new();
        for item in rows {
            items.push(item?);
        }

        Ok(items)
    }

    /// Find change history by issue key with optional field filter
    pub fn find_by_issue_key_and_field(
        &self,
        issue_key: &str,
        field_filter: Option<&str>,
    ) -> Result<Vec<ChangeHistoryItem>> {
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

        let mut stmt = conn.prepare(sql)?;

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
                    changed_at: row.get::<_, String>(11)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
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
                    changed_at: row.get::<_, String>(11)?
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect()
        };

        let mut items = Vec::new();
        for item in rows {
            items.push(item?);
        }

        Ok(items)
    }

    /// Count change history items by issue key
    pub fn count_by_issue_key(&self, issue_key: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issue_change_history WHERE issue_key = ?",
            duckdb::params![issue_key],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }
}
