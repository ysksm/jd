use crate::domain::entities::JiraField;
use crate::domain::error::{DomainError, DomainResult};
use chrono::Utc;
use duckdb::Connection;
use log::info;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct DuckDbIssuesExpandedRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DuckDbIssuesExpandedRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get existing columns in issues_expanded table
    pub fn get_existing_columns(&self) -> DomainResult<HashSet<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT column_name
                FROM information_schema.columns
                WHERE table_name = 'issues_expanded'
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut columns = HashSet::new();
        for col in rows {
            columns.insert(
                col.map_err(|e| DomainError::Repository(e.to_string()))?
                    .to_lowercase(),
            );
        }
        Ok(columns)
    }

    /// Add columns to issues_expanded table based on field definitions
    pub fn add_field_columns(&self, fields: &[JiraField]) -> DomainResult<Vec<String>> {
        let existing_columns = self.get_existing_columns()?;
        let conn = self.conn.lock().unwrap();
        let mut added = Vec::new();

        // Base columns that already exist in the table
        let base_columns: HashSet<&str> = [
            "id",
            "project_id",
            "issue_key",
            "summary",
            "description",
            "status",
            "priority",
            "assignee",
            "reporter",
            "issue_type",
            "resolution",
            "labels",
            "components",
            "fix_versions",
            "sprint",
            "parent_key",
            "created_date",
            "updated_date",
            "synced_at",
        ]
        .into_iter()
        .collect();

        for field in fields {
            if !field.is_expandable() {
                continue;
            }

            let col_name = field.get_safe_column_name();

            // Skip if already a base column (by checking the field id against known base fields)
            let field_id_lower = field.id.to_lowercase();
            if base_columns.contains(field_id_lower.as_str()) {
                continue;
            }

            // Map known field IDs to base columns
            let known_mappings: &[(&str, &str)] = &[
                ("summary", "summary"),
                ("description", "description"),
                ("status", "status"),
                ("priority", "priority"),
                ("assignee", "assignee"),
                ("reporter", "reporter"),
                ("issuetype", "issue_type"),
                ("resolution", "resolution"),
                ("labels", "labels"),
                ("components", "components"),
                ("fixversions", "fix_versions"),
                ("parent", "parent_key"),
                ("created", "created_date"),
                ("updated", "updated_date"),
            ];

            let is_base_mapping = known_mappings
                .iter()
                .any(|(k, _)| k == &field_id_lower.as_str());
            if is_base_mapping {
                continue;
            }

            // Skip if column already exists
            if existing_columns.contains(&col_name) {
                continue;
            }

            let col_type = field.get_column_type();
            let sql = format!(
                "ALTER TABLE issues_expanded ADD COLUMN \"{}\" {}",
                col_name, col_type
            );

            match conn.execute(&sql, []) {
                Ok(_) => {
                    info!(
                        "Added column {} ({}) to issues_expanded",
                        col_name, col_type
                    );
                    added.push(col_name);
                }
                Err(e) => {
                    // Ignore "column already exists" errors
                    let err_str = e.to_string();
                    if !err_str.contains("already exists") && !err_str.contains("duplicate column")
                    {
                        return Err(DomainError::Repository(format!(
                            "Failed to add column {}: {}",
                            col_name, e
                        )));
                    }
                }
            }
        }

        Ok(added)
    }

    /// Expand raw_data from issues table into issues_expanded table
    pub fn expand_issues(&self, project_id: Option<&str>) -> DomainResult<usize> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // First, check if issues table has data
        let issues_count: i64 = {
            let sql = match project_id {
                Some(pid) => format!(
                    "SELECT COUNT(*) FROM issues WHERE project_id = '{}' AND raw_data IS NOT NULL",
                    pid
                ),
                None => "SELECT COUNT(*) FROM issues WHERE raw_data IS NOT NULL".to_string(),
            };
            let mut stmt = conn.prepare(&sql).map_err(|e| {
                DomainError::Repository(format!("Failed to prepare count query: {}", e))
            })?;
            stmt.query_row([], |row| row.get(0))
                .map_err(|e| DomainError::Repository(format!("Failed to count issues: {}", e)))?
        };

        info!(
            "Found {} issues with raw_data to expand (project_id: {:?})",
            issues_count, project_id
        );

        if issues_count == 0 {
            info!("No issues with raw_data found to expand");
            return Ok(0);
        }

        // Get the list of columns we need to handle
        let columns = self.get_existing_columns_internal(&conn)?;
        info!("issues_expanded table has {} columns", columns.len());

        // Build the SELECT clause for extracting data from raw_data JSON
        let mut select_parts: Vec<String> = Vec::new();
        let mut column_names: Vec<&str> = Vec::new();

        // Core fields - directly from raw_data.fields
        let core_mappings = vec![
            ("id", "i.id"),
            ("project_id", "i.project_id"),
            (
                "issue_key",
                "COALESCE(i.raw_data->>'key', i.key) AS issue_key",
            ),
            (
                "summary",
                "COALESCE(i.raw_data->'fields'->>'summary', i.summary) AS summary",
            ),
            (
                "description",
                "i.raw_data->'fields'->>'description' AS description",
            ),
            (
                "status",
                "COALESCE(i.raw_data->'fields'->'status'->>'name', i.status) AS status",
            ),
            (
                "priority",
                "COALESCE(i.raw_data->'fields'->'priority'->>'name', i.priority) AS priority",
            ),
            (
                "assignee",
                "COALESCE(i.raw_data->'fields'->'assignee'->>'displayName', i.assignee) AS assignee",
            ),
            (
                "reporter",
                "COALESCE(i.raw_data->'fields'->'reporter'->>'displayName', i.reporter) AS reporter",
            ),
            (
                "issue_type",
                "COALESCE(i.raw_data->'fields'->'issuetype'->>'name', i.issue_type) AS issue_type",
            ),
            (
                "resolution",
                "i.raw_data->'fields'->'resolution'->>'name' AS resolution",
            ),
            ("labels", "i.raw_data->'fields'->'labels' AS labels"),
            (
                "components",
                "i.raw_data->'fields'->'components' AS components",
            ),
            (
                "fix_versions",
                "i.raw_data->'fields'->'fixVersions' AS fix_versions",
            ),
            ("sprint", "i.sprint"),
            (
                "parent_key",
                "i.raw_data->'fields'->'parent'->>'key' AS parent_key",
            ),
            (
                "created_date",
                "CAST(i.raw_data->'fields'->>'created' AS TIMESTAMP) AS created_date",
            ),
            (
                "updated_date",
                "CAST(i.raw_data->'fields'->>'updated' AS TIMESTAMP) AS updated_date",
            ),
        ];

        for (col, expr) in &core_mappings {
            column_names.push(*col);
            select_parts.push(expr.to_string());
        }

        // Add custom field columns
        for col in &columns {
            if col.starts_with("customfield_") {
                column_names.push(col.as_str());
                let expr = format!(
                    "CASE
                        WHEN json_type(i.raw_data->'fields'->'{col}') = 'OBJECT'
                        THEN COALESCE(
                            i.raw_data->'fields'->'{col}'->>'name',
                            i.raw_data->'fields'->'{col}'->>'value',
                            i.raw_data->'fields'->'{col}'->>'displayName',
                            CAST(i.raw_data->'fields'->'{col}' AS TEXT)
                        )
                        WHEN json_type(i.raw_data->'fields'->'{col}') = 'ARRAY'
                        THEN CAST(i.raw_data->'fields'->'{col}' AS TEXT)
                        ELSE i.raw_data->'fields'->>'{col}'
                    END AS \"{col}\""
                );
                select_parts.push(expr);
            }
        }

        // Build the INSERT query with ON CONFLICT for DuckDB
        let column_list = column_names.join(", ");
        let select_list = select_parts.join(",\n    ");

        let where_clause = match project_id {
            Some(pid) => format!("WHERE i.project_id = '{}'", pid),
            None => String::new(),
        };

        // Build the UPDATE SET clause for ON CONFLICT
        let update_set: Vec<String> = column_names
            .iter()
            .filter(|&&col| col != "id") // Don't update the primary key
            .map(|&col| format!("{} = excluded.{}", col, col))
            .collect();
        let update_set_clause = update_set.join(", ");

        let sql = format!(
            r#"
            INSERT INTO issues_expanded ({column_list}, synced_at)
            SELECT
                {select_list},
                '{now}' AS synced_at
            FROM issues i
            {where_clause}
            ON CONFLICT (id) DO UPDATE SET
                {update_set_clause},
                synced_at = excluded.synced_at
            "#
        );

        info!(
            "Executing expand_issues SQL with {} columns",
            column_names.len()
        );
        log::debug!("Generated SQL:\n{}", sql);

        let affected = conn.execute(&sql, []).map_err(|e| {
            log::error!("Failed to expand issues. SQL: {}", sql);
            DomainError::Repository(format!("Failed to expand issues: {}", e))
        })?;

        info!("expand_issues affected {} rows", affected);

        // Verify data was inserted
        let final_count: i64 = {
            let count_sql = match project_id {
                Some(pid) => format!(
                    "SELECT COUNT(*) FROM issues_expanded WHERE project_id = '{}'",
                    pid
                ),
                None => "SELECT COUNT(*) FROM issues_expanded".to_string(),
            };
            let mut stmt = conn.prepare(&count_sql).map_err(|e| {
                DomainError::Repository(format!("Failed to prepare count query: {}", e))
            })?;
            stmt.query_row([], |row| row.get(0))
                .map_err(|e| DomainError::Repository(format!("Failed to count: {}", e)))?
        };
        info!("issues_expanded now has {} total rows", final_count);

        Ok(affected)
    }

    /// Get count of expanded issues
    pub fn count(&self, project_id: Option<&str>) -> DomainResult<i64> {
        let conn = self.conn.lock().unwrap();

        let sql = match project_id {
            Some(_) => "SELECT COUNT(*) FROM issues_expanded WHERE project_id = ?",
            None => "SELECT COUNT(*) FROM issues_expanded",
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let count: i64 = match project_id {
            Some(pid) => stmt.query_row([pid], |row| row.get(0)),
            None => stmt.query_row([], |row| row.get(0)),
        }
        .map_err(|e| DomainError::Repository(format!("Failed to count expanded issues: {}", e)))?;

        Ok(count)
    }

    /// Delete all expanded issues for a project
    pub fn delete_by_project(&self, project_id: &str) -> DomainResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM issues_expanded WHERE project_id = ?",
            [project_id],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to delete expanded issues: {}", e)))?;
        Ok(())
    }

    /// Helper to get existing columns without acquiring lock
    fn get_existing_columns_internal(&self, conn: &Connection) -> DomainResult<HashSet<String>> {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT column_name
                FROM information_schema.columns
                WHERE table_name = 'issues_expanded'
                "#,
            )
            .map_err(|e| DomainError::Repository(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| DomainError::Repository(format!("Failed to execute query: {}", e)))?;

        let mut columns = HashSet::new();
        for col in rows {
            columns.insert(
                col.map_err(|e| DomainError::Repository(e.to_string()))?
                    .to_lowercase(),
            );
        }
        Ok(columns)
    }
}
