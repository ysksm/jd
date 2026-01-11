//! SQL command handlers

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Utc;
use duckdb::Connection;
use tauri::State;
use uuid::Uuid;

use jira_db_core::ExecuteSqlUseCase;

use crate::generated::*;
use crate::state::AppState;

const SAVED_QUERIES_FILE: &str = "./data/saved_queries.json";

/// Execute a SQL query (read-only)
#[tauri::command]
pub async fn sql_execute(
    state: State<'_, AppState>,
    request: SqlExecuteRequest,
) -> Result<SqlExecuteResponse, String> {
    let start = Instant::now();

    // Check if we should query all projects
    if request.all_projects.unwrap_or(false) {
        return execute_all_projects_query(&state, &request, start).await;
    }

    // Single project query (default behavior)
    let project_key = request.project_key.as_deref().unwrap_or("");
    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    // Use core use case for SQL execution
    let use_case = ExecuteSqlUseCase::new(db);
    let result = use_case
        .execute(&request.query, request.limit.map(|l| l as usize))
        .map_err(|e| e.to_string())?;

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Convert rows to named objects
    let rows: Vec<serde_json::Value> = result
        .rows
        .into_iter()
        .map(|row_values| {
            let mut row_data = serde_json::Map::new();
            for (i, value) in row_values.into_iter().enumerate() {
                let col_name = result
                    .columns
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                row_data.insert(col_name, value);
            }
            serde_json::Value::Object(row_data)
        })
        .collect();

    Ok(SqlExecuteResponse {
        columns: result.columns,
        rows,
        row_count: result.row_count as i32,
        execution_time_ms,
    })
}

/// Execute query across all synced projects
async fn execute_all_projects_query(
    state: &State<'_, AppState>,
    request: &SqlExecuteRequest,
    start: Instant,
) -> Result<SqlExecuteResponse, String> {
    // Get enabled projects from settings
    let settings = state
        .get_settings()
        .ok_or_else(|| "Settings not initialized".to_string())?;

    let enabled_projects: Vec<String> = settings
        .sync_enabled_projects()
        .iter()
        .map(|p| p.key.clone())
        .collect();

    if enabled_projects.is_empty() {
        return Err("No enabled projects found".to_string());
    }

    // Get database factory to access project database paths
    let factory = state
        .get_db_factory()
        .ok_or_else(|| "Database factory not initialized".to_string())?;

    // Create an in-memory database for cross-project queries
    let conn = Connection::open_in_memory()
        .map_err(|e| format!("Failed to create in-memory database: {}", e))?;

    // Attach all project databases
    for project_key in &enabled_projects {
        let db_path = factory.get_database_path(project_key);

        if db_path.exists() {
            let attach_sql = format!(
                "ATTACH '{}' AS {} (READ_ONLY)",
                db_path.to_string_lossy().replace('\'', "''"),
                sanitize_identifier(project_key)
            );
            conn.execute(&attach_sql, [])
                .map_err(|e| format!("Failed to attach database {}: {}", project_key, e))?;
        }
    }

    // Create union views for common tables
    create_union_views(&conn, &enabled_projects)?;

    // Execute the query using the core use case
    let db = Arc::new(Mutex::new(conn));
    let use_case = ExecuteSqlUseCase::new(db);
    let result = use_case
        .execute(&request.query, request.limit.map(|l| l as usize))
        .map_err(|e| e.to_string())?;

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Convert rows to named objects
    let rows: Vec<serde_json::Value> = result
        .rows
        .into_iter()
        .map(|row_values| {
            let mut row_data = serde_json::Map::new();
            for (i, value) in row_values.into_iter().enumerate() {
                let col_name = result
                    .columns
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                row_data.insert(col_name, value);
            }
            serde_json::Value::Object(row_data)
        })
        .collect();

    Ok(SqlExecuteResponse {
        columns: result.columns,
        rows,
        row_count: result.row_count as i32,
        execution_time_ms,
    })
}

/// Create UNION ALL views for common tables across all attached databases
fn create_union_views(conn: &Connection, project_keys: &[String]) -> Result<(), String> {
    // Tables to create union views for
    let tables = [
        "issues",
        "projects",
        "statuses",
        "priorities",
        "issue_types",
        "labels",
        "components",
        "fix_versions",
        "issue_change_history",
        "issue_snapshots",
    ];

    for table in &tables {
        let unions: Vec<String> = project_keys
            .iter()
            .map(|key| format!("SELECT * FROM {}.{}", sanitize_identifier(key), table))
            .collect();

        if !unions.is_empty() {
            let view_sql = format!(
                "CREATE OR REPLACE VIEW {} AS {}",
                table,
                unions.join(" UNION ALL ")
            );

            // Try to create the view, but don't fail if the table doesn't exist in some databases
            if let Err(e) = conn.execute(&view_sql, []) {
                tracing::debug!(
                    "Could not create union view for table {}: {} (table may not exist in all databases)",
                    table,
                    e
                );
            }
        }
    }

    Ok(())
}

/// Sanitize a project key to be used as a database identifier
fn sanitize_identifier(key: &str) -> String {
    // Replace non-alphanumeric characters with underscores
    let sanitized: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Ensure it doesn't start with a number
    if sanitized.chars().next().map_or(false, |c| c.is_numeric()) {
        format!("p_{}", sanitized)
    } else {
        sanitized
    }
}

/// Get database schema
#[tauri::command]
pub async fn sql_get_schema(
    state: State<'_, AppState>,
    request: SqlGetSchemaRequest,
) -> Result<SqlGetSchemaResponse, String> {
    // Check if we should show schema for all projects
    if request.all_projects.unwrap_or(false) {
        return get_all_projects_schema(&state, &request).await;
    }

    // Single project schema (default behavior)
    let project_key = request.project_key.as_deref().unwrap_or("");
    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    let conn = db
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    get_schema_from_connection(&conn, request.table)
}

/// Get schema for all projects combined
async fn get_all_projects_schema(
    state: &State<'_, AppState>,
    request: &SqlGetSchemaRequest,
) -> Result<SqlGetSchemaResponse, String> {
    // Get enabled projects from settings
    let settings = state
        .get_settings()
        .ok_or_else(|| "Settings not initialized".to_string())?;

    let enabled_projects: Vec<String> = settings
        .sync_enabled_projects()
        .iter()
        .map(|p| p.key.clone())
        .collect();

    if enabled_projects.is_empty() {
        return Err("No enabled projects found".to_string());
    }

    // Get database factory to access project database paths
    let factory = state
        .get_db_factory()
        .ok_or_else(|| "Database factory not initialized".to_string())?;

    // Create an in-memory database
    let conn = Connection::open_in_memory()
        .map_err(|e| format!("Failed to create in-memory database: {}", e))?;

    // Attach all project databases
    for project_key in &enabled_projects {
        let db_path = factory.get_database_path(project_key);

        if db_path.exists() {
            let attach_sql = format!(
                "ATTACH '{}' AS {} (READ_ONLY)",
                db_path.to_string_lossy().replace('\'', "''"),
                sanitize_identifier(project_key)
            );
            conn.execute(&attach_sql, [])
                .map_err(|e| format!("Failed to attach database {}: {}", project_key, e))?;
        }
    }

    // Create union views for common tables
    create_union_views(&conn, &enabled_projects)?;

    get_schema_from_connection(&conn, request.table.clone())
}

/// Get schema from a database connection
fn get_schema_from_connection(
    conn: &Connection,
    table: Option<String>,
) -> Result<SqlGetSchemaResponse, String> {
    if let Some(table_name) = table {
        // Get columns for a specific table
        let query = format!(
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
            table_name.replace('\'', "''")
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Schema query error: {}", e))?;

        let columns: Vec<SqlColumn> = stmt
            .query_map([], |row| {
                Ok(SqlColumn {
                    name: row.get(0)?,
                    data_type: row.get(1)?,
                    is_nullable: row.get::<_, String>(2)? == "YES",
                })
            })
            .map_err(|e| format!("Schema query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(SqlGetSchemaResponse {
            tables: vec![SqlTable {
                name: table_name,
                columns: Some(columns),
            }],
        })
    } else {
        // Get all tables
        let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| format!("Schema query error: {}", e))?;

        let tables: Vec<SqlTable> = stmt
            .query_map([], |row| {
                Ok(SqlTable {
                    name: row.get(0)?,
                    columns: None,
                })
            })
            .map_err(|e| format!("Schema query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(SqlGetSchemaResponse { tables })
    }
}

/// Load saved queries from file
fn load_saved_queries() -> Vec<SavedQuery> {
    let path = PathBuf::from(SAVED_QUERIES_FILE);
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Save queries to file
fn save_queries_to_file(queries: &[SavedQuery]) -> Result<(), String> {
    let path = PathBuf::from(SAVED_QUERIES_FILE);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(queries)
        .map_err(|e| format!("Failed to serialize queries: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to save queries: {}", e))?;
    Ok(())
}

/// List saved queries
#[tauri::command]
pub async fn sql_list_queries(
    _request: SqlQueryListRequest,
) -> Result<SqlQueryListResponse, String> {
    let queries = load_saved_queries();
    Ok(SqlQueryListResponse { queries })
}

/// Save a query
#[tauri::command]
pub async fn sql_save_query(request: SqlQuerySaveRequest) -> Result<SqlQuerySaveResponse, String> {
    let mut queries = load_saved_queries();
    let now = Utc::now();

    let query = if let Some(id) = request.id {
        // Update existing query
        if let Some(existing) = queries.iter_mut().find(|q| q.id == id) {
            existing.name = request.name;
            existing.query = request.query;
            existing.description = request.description;
            existing.updated_at = now;
            existing.clone()
        } else {
            return Err(format!("Query with id {} not found", id));
        }
    } else {
        // Create new query
        let new_query = SavedQuery {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            query: request.query,
            description: request.description,
            created_at: now,
            updated_at: now,
        };
        queries.push(new_query.clone());
        new_query
    };

    save_queries_to_file(&queries)?;

    Ok(SqlQuerySaveResponse { query })
}

/// Delete a saved query
#[tauri::command]
pub async fn sql_delete_query(
    request: SqlQueryDeleteRequest,
) -> Result<SqlQueryDeleteResponse, String> {
    let mut queries = load_saved_queries();
    let initial_len = queries.len();
    queries.retain(|q| q.id != request.id);

    if queries.len() == initial_len {
        return Err(format!("Query with id {} not found", request.id));
    }

    save_queries_to_file(&queries)?;

    Ok(SqlQueryDeleteResponse { success: true })
}
