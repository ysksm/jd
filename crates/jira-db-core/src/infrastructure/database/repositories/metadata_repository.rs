use crate::domain::entities::{Component, FixVersion, IssueType, Label, Priority, Status};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::MetadataRepository;
use chrono::{DateTime, Utc};
use duckdb::Connection;
use log::debug;
use std::sync::{Arc, Mutex};

pub struct DuckDbMetadataRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbMetadataRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

impl MetadataRepository for DuckDbMetadataRepository {
    fn upsert_statuses(&self, project_id: &str, statuses: &[Status]) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        debug!(
            "[SQL] Upserting {} statuses for project {}",
            statuses.len(),
            project_id
        );

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
            ).map_err(|e| DomainError::Repository(format!("Failed to upsert status: {}", e)))?;
        }
        Ok(())
    }

    fn find_statuses_by_project(&self, project_id: &str) -> DomainResult<Vec<Status>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, description, category FROM statuses WHERE project_id = ? ORDER BY name")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(Status {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    category: row.get(2)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut statuses = Vec::new();
        for status in rows {
            statuses.push(status.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(statuses)
    }

    fn upsert_priorities(&self, project_id: &str, priorities: &[Priority]) -> DomainResult<()> {
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
            ).map_err(|e| DomainError::Repository(format!("Failed to upsert priority: {}", e)))?;
        }
        Ok(())
    }

    fn find_priorities_by_project(&self, project_id: &str) -> DomainResult<Vec<Priority>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, description, icon_url FROM priorities WHERE project_id = ? ORDER BY name")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(Priority {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    icon_url: row.get(2)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut priorities = Vec::new();
        for priority in rows {
            priorities.push(priority.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(priorities)
    }

    fn upsert_issue_types(&self, project_id: &str, issue_types: &[IssueType]) -> DomainResult<()> {
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
            ).map_err(|e| DomainError::Repository(format!("Failed to upsert issue type: {}", e)))?;
        }
        Ok(())
    }

    fn find_issue_types_by_project(&self, project_id: &str) -> DomainResult<Vec<IssueType>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, description, icon_url, subtask FROM issue_types WHERE project_id = ? ORDER BY name")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(IssueType {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    icon_url: row.get(2)?,
                    subtask: row.get(3)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut issue_types = Vec::new();
        for issue_type in rows {
            issue_types.push(issue_type.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(issue_types)
    }

    fn upsert_labels(&self, project_id: &str, labels: &[Label]) -> DomainResult<()> {
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
                duckdb::params![project_id, &label.name, &now, &now],
            )
            .map_err(|e| DomainError::Repository(format!("Failed to upsert label: {}", e)))?;
        }
        Ok(())
    }

    fn find_labels_by_project(&self, project_id: &str) -> DomainResult<Vec<Label>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM labels WHERE project_id = ? ORDER BY name")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(Label { name: row.get(0)? })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut labels = Vec::new();
        for label in rows {
            labels.push(label.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(labels)
    }

    fn upsert_components(&self, project_id: &str, components: &[Component]) -> DomainResult<()> {
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
            )
            .map_err(|e| DomainError::Repository(format!("Failed to upsert component: {}", e)))?;
        }
        Ok(())
    }

    fn find_components_by_project(&self, project_id: &str) -> DomainResult<Vec<Component>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT name, description, lead FROM components WHERE project_id = ? ORDER BY name",
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(Component {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    lead: row.get(2)?,
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut components = Vec::new();
        for component in rows {
            components.push(component.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(components)
    }

    fn upsert_fix_versions(
        &self,
        project_id: &str,
        fix_versions: &[FixVersion],
    ) -> DomainResult<()> {
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
            ).map_err(|e| DomainError::Repository(format!("Failed to upsert fix version: {}", e)))?;
        }
        Ok(())
    }

    fn find_fix_versions_by_project(&self, project_id: &str) -> DomainResult<Vec<FixVersion>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, description, released, release_date FROM fix_versions WHERE project_id = ? ORDER BY name")
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(duckdb::params![project_id], |row| {
                Ok(FixVersion {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    released: row.get(2)?,
                    release_date: row.get::<_, Option<String>>(3)?.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                })
            })
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut fix_versions = Vec::new();
        for version in rows {
            fix_versions.push(version.map_err(|e| DomainError::Repository(e.to_string()))?);
        }
        Ok(fix_versions)
    }
}
