//! SQL command handlers

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use jira_db_core::ExecuteSqlUseCase;

use crate::generated::*;
use crate::state::AppState;

const SAVED_QUERIES_FILE: &str = "./saved_queries.json";

/// Execute a SQL query (read-only)
#[tauri::command]
pub async fn sql_execute(
    state: State<'_, AppState>,
    request: SqlExecuteRequest,
) -> Result<SqlExecuteResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    let start = Instant::now();

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

/// Get database schema
#[tauri::command]
pub async fn sql_get_schema(
    state: State<'_, AppState>,
    request: SqlGetSchemaRequest,
) -> Result<SqlGetSchemaResponse, String> {
    let db = state.get_db().ok_or("Database not initialized")?;

    let conn = db
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    if let Some(table_name) = request.table {
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
    let json = serde_json::to_string_pretty(queries)
        .map_err(|e| format!("Failed to serialize queries: {}", e))?;
    fs::write(SAVED_QUERIES_FILE, json).map_err(|e| format!("Failed to save queries: {}", e))?;
    Ok(())
}

/// List saved queries
#[tauri::command]
pub async fn sql_query_list(_request: SqlQueryListRequest) -> Result<SqlQueryListResponse, String> {
    let queries = load_saved_queries();
    Ok(SqlQueryListResponse { queries })
}

/// Save a query
#[tauri::command]
pub async fn sql_query_save(request: SqlQuerySaveRequest) -> Result<SqlQuerySaveResponse, String> {
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
pub async fn sql_query_delete(
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
