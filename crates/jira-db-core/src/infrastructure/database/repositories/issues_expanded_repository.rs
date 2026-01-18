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
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;
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
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;
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
            "creator",
            "issue_type",
            "resolution",
            "labels",
            "components",
            "fix_versions",
            "affected_versions",
            "sprint",
            "parent_key",
            "environment",
            "security_level",
            "created_date",
            "updated_date",
            "resolved_date",
            "due_date",
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
                ("creator", "creator"),
                ("issuetype", "issue_type"),
                ("resolution", "resolution"),
                ("labels", "labels"),
                ("components", "components"),
                ("fixversions", "fix_versions"),
                ("versions", "affected_versions"),
                ("parent", "parent_key"),
                ("environment", "environment"),
                ("security", "security_level"),
                ("created", "created_date"),
                ("updated", "updated_date"),
                ("resolutiondate", "resolved_date"),
                ("duedate", "due_date"),
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
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;
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
                "creator",
                "i.raw_data->'fields'->'creator'->>'displayName' AS creator",
            ),
            (
                "issue_type",
                "COALESCE(i.raw_data->'fields'->'issuetype'->>'name', i.issue_type) AS issue_type",
            ),
            (
                "resolution",
                "i.raw_data->'fields'->'resolution'->>'name' AS resolution",
            ),
            (
                "labels",
                "TRY_CAST(i.raw_data->'fields'->'labels' AS JSON) AS labels",
            ),
            (
                "components",
                "TRY_CAST(i.raw_data->'fields'->'components' AS JSON) AS components",
            ),
            (
                "fix_versions",
                "TRY_CAST(i.raw_data->'fields'->'fixVersions' AS JSON) AS fix_versions",
            ),
            (
                "affected_versions",
                "TRY_CAST(i.raw_data->'fields'->'versions' AS JSON) AS affected_versions",
            ),
            ("sprint", "i.sprint"),
            (
                "parent_key",
                "i.raw_data->'fields'->'parent'->>'key' AS parent_key",
            ),
            (
                "environment",
                "i.raw_data->'fields'->>'environment' AS environment",
            ),
            (
                "security_level",
                "i.raw_data->'fields'->'security'->>'name' AS security_level",
            ),
            (
                "created_date",
                "TRY_CAST(i.raw_data->'fields'->>'created' AS TIMESTAMP) AS created_date",
            ),
            (
                "updated_date",
                "TRY_CAST(i.raw_data->'fields'->>'updated' AS TIMESTAMP) AS updated_date",
            ),
            (
                "resolved_date",
                "TRY_CAST(i.raw_data->'fields'->>'resolutiondate' AS TIMESTAMP) AS resolved_date",
            ),
            (
                "due_date",
                "TRY_CAST(i.raw_data->'fields'->>'duedate' AS TIMESTAMP) AS due_date",
            ),
        ];

        for (col, expr) in &core_mappings {
            column_names.push(*col);
            select_parts.push(expr.to_string());
        }

        // Add custom field columns
        // Simplified extraction: try to get common properties, fallback to text representation
        for col in &columns {
            if col.starts_with("customfield_") {
                column_names.push(col.as_str());
                // Try to extract name/value/displayName for objects, or just the text value
                // Using COALESCE to handle different field structures
                let expr = format!(
                    "COALESCE(
                        i.raw_data->'fields'->'{col}'->>'name',
                        i.raw_data->'fields'->'{col}'->>'value',
                        i.raw_data->'fields'->'{col}'->>'displayName',
                        i.raw_data->'fields'->>'{col}'
                    ) AS \"{col}\""
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
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

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
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;
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

    /// Create or replace a view with human-readable column names
    /// The view maps column names from issues_expanded to jira_fields.name
    pub fn create_readable_view(&self, fields: &[JiraField]) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        // Get existing columns in issues_expanded
        let columns = self.get_existing_columns_internal(&conn)?;

        // Create a map from field id (lowercase) to field name
        // This includes both base fields (like "summary", "status") and custom fields
        let field_name_map: std::collections::HashMap<String, String> = fields
            .iter()
            .map(|f| (f.id.to_lowercase(), f.name.clone()))
            .collect();

        // Mapping from column name to JIRA field id (for base columns with different names)
        let column_to_field_id: std::collections::HashMap<&str, &str> = [
            ("issue_type", "issuetype"),
            ("fix_versions", "fixversions"),
            ("affected_versions", "versions"),
            ("parent_key", "parent"),
            ("security_level", "security"),
            ("created_date", "created"),
            ("updated_date", "updated"),
            ("resolved_date", "resolutiondate"),
            ("due_date", "duedate"),
        ]
        .into_iter()
        .collect();

        // Internal columns that don't come from JIRA fields (use column name as-is)
        let internal_columns: std::collections::HashSet<&str> =
            ["id", "project_id", "issue_key", "synced_at"]
                .into_iter()
                .collect();

        // Build column aliases dynamically
        let mut column_aliases: Vec<(String, String)> = Vec::new();

        // Define column order for consistency
        let base_column_order = [
            "id",
            "project_id",
            "issue_key",
            "summary",
            "description",
            "status",
            "priority",
            "assignee",
            "reporter",
            "creator",
            "issue_type",
            "resolution",
            "labels",
            "components",
            "fix_versions",
            "affected_versions",
            "sprint",
            "parent_key",
            "environment",
            "security_level",
            "created_date",
            "updated_date",
            "resolved_date",
            "due_date",
            "synced_at",
        ];

        // Add base columns in order
        for col in base_column_order {
            if columns.contains(col) {
                let display_name = if internal_columns.contains(col) {
                    // Internal columns: use column name as-is
                    col.to_string()
                } else {
                    // Look up field name from jira_fields
                    let field_id = column_to_field_id.get(col).copied().unwrap_or(col);
                    field_name_map
                        .get(field_id)
                        .cloned()
                        .unwrap_or_else(|| col.to_string())
                };
                column_aliases.push((col.to_string(), display_name));
            }
        }

        // Add custom field columns with names from jira_fields
        for col in &columns {
            if col.starts_with("customfield_") || col.starts_with("cf_") {
                let display_name = field_name_map
                    .get(col)
                    .cloned()
                    .unwrap_or_else(|| col.clone());
                column_aliases.push((col.clone(), display_name));
            } else if !base_column_order.contains(&col.as_str()) {
                // Other columns not in base order - try to find a readable name
                let display_name = field_name_map
                    .get(col)
                    .cloned()
                    .unwrap_or_else(|| col.clone());
                column_aliases.push((col.clone(), display_name));
            }
        }

        // Build the SELECT clause with aliases
        let select_parts: Vec<String> = column_aliases
            .iter()
            .map(|(col, display)| {
                // Escape quotes in display name and use as alias
                let escaped_display = display.replace('"', "\"\"");
                format!("\"{}\" AS \"{}\"", col, escaped_display)
            })
            .collect();

        let select_clause = select_parts.join(",\n    ");

        // Create the view
        let sql = format!(
            r#"
            CREATE OR REPLACE VIEW issues_readable AS
            SELECT
                {select_clause}
            FROM issues_expanded
            "#
        );

        conn.execute(&sql, []).map_err(|e| {
            log::error!("Failed to create view. SQL: {}", sql);
            DomainError::Repository(format!("Failed to create readable view: {}", e))
        })?;

        info!(
            "Created issues_readable view with {} columns",
            column_aliases.len()
        );

        Ok(())
    }

    /// Create or replace a view for issue_snapshots with human-readable column names
    pub fn create_snapshots_readable_view(&self, fields: &[JiraField]) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire database lock: {}", e))
        })?;

        // Create a map from field id (lowercase) to field name
        let field_name_map: std::collections::HashMap<String, String> = fields
            .iter()
            .map(|f| (f.id.to_lowercase(), f.name.clone()))
            .collect();

        // Mapping from column name to JIRA field id (for columns with different names)
        let column_to_field_id: std::collections::HashMap<&str, &str> = [
            ("issue_type", "issuetype"),
            ("fix_versions", "fixversions"),
            ("parent_key", "parent"),
            ("resolved_date", "resolutiondate"),
            ("due_date", "duedate"),
            ("updated_date", "updated"),
        ]
        .into_iter()
        .collect();

        // Internal columns that don't come from JIRA fields (use column name as-is)
        let internal_columns: std::collections::HashSet<&str> = [
            "issue_id",
            "issue_key",
            "project_id",
            "version",
            "valid_from",
            "valid_to",
            "created_at",
        ]
        .into_iter()
        .collect();

        // Define column order
        let snapshot_columns = [
            "issue_id",
            "issue_key",
            "project_id",
            "version",
            "valid_from",
            "valid_to",
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
            "resolved_date",
            "due_date",
            "updated_date",
            "created_at",
        ];

        // Build column aliases dynamically
        let column_aliases: Vec<(String, String)> = snapshot_columns
            .iter()
            .map(|&col| {
                let display_name = if internal_columns.contains(col) {
                    col.to_string()
                } else {
                    let field_id = column_to_field_id.get(col).copied().unwrap_or(col);
                    field_name_map
                        .get(field_id)
                        .cloned()
                        .unwrap_or_else(|| col.to_string())
                };
                (col.to_string(), display_name)
            })
            .collect();

        // Build the SELECT clause with aliases
        let select_parts: Vec<String> = column_aliases
            .iter()
            .map(|(col, display)| {
                let escaped_display = display.replace('"', "\"\"");
                format!("s.\"{}\" AS \"{}\"", col, escaped_display)
            })
            .collect();

        let select_clause = select_parts.join(",\n    ");

        // Create the basic readable view
        let sql = format!(
            r#"
            CREATE OR REPLACE VIEW issue_snapshots_readable AS
            SELECT
                {select_clause}
            FROM issue_snapshots s
            "#
        );

        conn.execute(&sql, []).map_err(|e| {
            log::error!("Failed to create view. SQL: {}", sql);
            DomainError::Repository(format!("Failed to create snapshots readable view: {}", e))
        })?;

        info!(
            "Created issue_snapshots_readable view with {} columns",
            column_aliases.len()
        );

        // Create expanded view for current snapshots (valid_to IS NULL)
        // This joins with issues.raw_data to get custom field values
        self.create_snapshots_expanded_view_internal(&conn, fields)?;

        Ok(())
    }

    /// Create an expanded view for current snapshots with custom fields from issue_snapshots.raw_data
    fn create_snapshots_expanded_view_internal(
        &self,
        conn: &Connection,
        fields: &[JiraField],
    ) -> DomainResult<()> {
        // Base snapshot columns
        // For resolved_date, due_date, and updated_date, fallback to raw_data extraction if column is NULL
        let base_columns = vec![
            ("s.issue_id", "issue_id"),
            ("s.issue_key", "issue_key"),
            ("s.project_id", "project_id"),
            ("s.version", "version"),
            ("s.valid_from", "valid_from"),
            ("s.valid_to", "valid_to"),
            ("s.summary", "summary"),
            ("s.description", "description"),
            ("s.status", "status"),
            ("s.priority", "priority"),
            ("s.assignee", "assignee"),
            ("s.reporter", "reporter"),
            ("s.issue_type", "issue_type"),
            ("s.resolution", "resolution"),
            ("s.labels", "labels"),
            ("s.components", "components"),
            ("s.fix_versions", "fix_versions"),
            ("s.sprint", "sprint"),
            ("s.parent_key", "parent_key"),
            (
                "COALESCE(s.resolved_date, TRY_CAST(s.raw_data->'fields'->>'resolutiondate' AS TIMESTAMP))",
                "resolved_date",
            ),
            (
                "COALESCE(s.due_date, TRY_CAST(s.raw_data->'fields'->>'duedate' AS TIMESTAMP))",
                "due_date",
            ),
            (
                "COALESCE(s.updated_date, TRY_CAST(s.raw_data->'fields'->>'updated' AS TIMESTAMP))",
                "updated_date",
            ),
            ("s.created_at", "created_at"),
        ];

        let mut select_parts: Vec<String> = base_columns
            .iter()
            .map(|(expr, alias)| format!("{} AS \"{}\"", expr, alias))
            .collect();

        // Add custom field columns from issues.raw_data
        // Only include expandable custom fields
        for field in fields {
            if !field.is_expandable() {
                continue;
            }

            // Skip base fields that are already included
            let field_id_lower = field.id.to_lowercase();
            let skip_fields = [
                "summary",
                "description",
                "status",
                "priority",
                "assignee",
                "reporter",
                "issuetype",
                "resolution",
                "labels",
                "components",
                "fixversions",
                "parent",
                "created",
                "updated",
                "resolutiondate",
                "duedate",
            ];

            if skip_fields.contains(&field_id_lower.as_str()) {
                continue;
            }

            if field.id.starts_with("customfield_") {
                let col_name = field.get_safe_column_name();
                // Use s.raw_data (from issue_snapshots) instead of joining with issues
                let expr = format!(
                    "COALESCE(
                        s.raw_data->'fields'->'{}'->>'name',
                        s.raw_data->'fields'->'{}'->>'value',
                        s.raw_data->'fields'->'{}'->>'displayName',
                        s.raw_data->'fields'->>'{}'
                    ) AS \"{}\"",
                    field.id, field.id, field.id, field.id, col_name
                );
                select_parts.push(expr);
            }
        }

        let select_clause = select_parts.join(",\n    ");

        // Create view using issue_snapshots.raw_data directly
        // raw_data is now available for all snapshots (reconstructed from changelog)
        let sql = format!(
            r#"
            CREATE OR REPLACE VIEW issue_snapshots_expanded AS
            SELECT
                {select_clause}
            FROM issue_snapshots s
            "#
        );

        conn.execute(&sql, []).map_err(|e| {
            log::error!("Failed to create expanded snapshots view. SQL: {}", sql);
            DomainError::Repository(format!("Failed to create snapshots expanded view: {}", e))
        })?;

        info!("Created issue_snapshots_expanded view with custom fields");

        // Create readable version of the expanded view
        self.create_snapshots_expanded_readable_view_internal(conn, fields)?;

        Ok(())
    }

    /// Create a readable version of issue_snapshots_expanded
    fn create_snapshots_expanded_readable_view_internal(
        &self,
        conn: &Connection,
        fields: &[JiraField],
    ) -> DomainResult<()> {
        // Create a map from field id (lowercase) to field name
        let field_name_map: std::collections::HashMap<String, String> = fields
            .iter()
            .map(|f| (f.id.to_lowercase(), f.name.clone()))
            .collect();

        // Mapping from column name to JIRA field id (for columns with different names)
        let column_to_field_id: std::collections::HashMap<&str, &str> = [
            ("issue_type", "issuetype"),
            ("fix_versions", "fixversions"),
            ("parent_key", "parent"),
            ("resolved_date", "resolutiondate"),
            ("due_date", "duedate"),
            ("updated_date", "updated"),
        ]
        .into_iter()
        .collect();

        // Internal columns that don't come from JIRA fields (use column name as-is)
        let internal_columns: std::collections::HashSet<&str> = [
            "issue_id",
            "issue_key",
            "project_id",
            "version",
            "valid_from",
            "valid_to",
            "created_at",
        ]
        .into_iter()
        .collect();

        // Define base column order
        let base_columns = [
            "issue_id",
            "issue_key",
            "project_id",
            "version",
            "valid_from",
            "valid_to",
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
            "resolved_date",
            "due_date",
            "updated_date",
            "created_at",
        ];

        // Build select parts for base columns
        let mut select_parts: Vec<String> = base_columns
            .iter()
            .map(|&col| {
                let display_name = if internal_columns.contains(col) {
                    col.to_string()
                } else {
                    let field_id = column_to_field_id.get(col).copied().unwrap_or(col);
                    field_name_map
                        .get(field_id)
                        .cloned()
                        .unwrap_or_else(|| col.to_string())
                };
                let escaped_display = display_name.replace('"', "\"\"");
                format!("\"{}\" AS \"{}\"", col, escaped_display)
            })
            .collect();

        // Add custom field columns with readable names
        for field in fields {
            if !field.is_expandable() {
                continue;
            }

            if field.id.starts_with("customfield_") {
                let col_name = field.get_safe_column_name();
                // Use field.name directly as it's the human-readable name from JIRA
                let escaped_display = field.name.replace('"', "\"\"");
                select_parts.push(format!("\"{}\" AS \"{}\"", col_name, escaped_display));
            }
        }

        let select_clause = select_parts.join(",\n    ");

        let sql = format!(
            r#"
            CREATE OR REPLACE VIEW issue_snapshots_expanded_readable AS
            SELECT
                {select_clause}
            FROM issue_snapshots_expanded
            "#
        );

        conn.execute(&sql, []).map_err(|e| {
            log::error!(
                "Failed to create readable expanded snapshots view. SQL: {}",
                sql
            );
            DomainError::Repository(format!(
                "Failed to create snapshots expanded readable view: {}",
                e
            ))
        })?;

        info!("Created issue_snapshots_expanded_readable view");

        Ok(())
    }
}
