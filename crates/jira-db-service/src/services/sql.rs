//! SQL service

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use jira_db_core::ExecuteSqlUseCase;

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

const SAVED_QUERIES_FILE: &str = "./data/saved_queries.json";

/// Execute a SQL query (read-only)
pub fn execute(state: &AppState, request: SqlExecuteRequest) -> ServiceResult<SqlExecuteResponse> {
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    let start = Instant::now();

    // Use core use case for SQL execution
    let use_case = ExecuteSqlUseCase::new(db);
    let result = use_case
        .execute(&request.query, request.limit.map(|l| l as usize))
        .map_err(|e| ServiceError::Database(e.to_string()))?;

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
pub fn get_schema(
    state: &AppState,
    request: SqlGetSchemaRequest,
) -> ServiceResult<SqlGetSchemaResponse> {
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    let conn = db
        .lock()
        .map_err(|e| ServiceError::Database(format!("Failed to lock connection: {}", e)))?;

    if let Some(table_name) = request.table {
        // Get columns for a specific table
        let query = format!(
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
            table_name.replace('\'', "''")
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| ServiceError::Database(format!("Schema query error: {}", e)))?;

        let columns: Vec<SqlColumn> = stmt
            .query_map([], |row| {
                Ok(SqlColumn {
                    name: row.get(0)?,
                    data_type: row.get(1)?,
                    is_nullable: row.get::<_, String>(2)? == "YES",
                })
            })
            .map_err(|e| ServiceError::Database(format!("Schema query error: {}", e)))?
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
            .map_err(|e| ServiceError::Database(format!("Schema query error: {}", e)))?;

        let tables: Vec<SqlTable> = stmt
            .query_map([], |row| {
                Ok(SqlTable {
                    name: row.get(0)?,
                    columns: None,
                })
            })
            .map_err(|e| ServiceError::Database(format!("Schema query error: {}", e)))?
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
fn save_queries_to_file(queries: &[SavedQuery]) -> ServiceResult<()> {
    let path = PathBuf::from(SAVED_QUERIES_FILE);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ServiceError::Io(format!("Failed to create data directory: {}", e)))?;
    }

    let json = serde_json::to_string_pretty(queries)
        .map_err(|e| ServiceError::Internal(format!("Failed to serialize queries: {}", e)))?;
    fs::write(&path, json)
        .map_err(|e| ServiceError::Io(format!("Failed to save queries: {}", e)))?;
    Ok(())
}

/// List saved queries
pub fn query_list(_request: SqlQueryListRequest) -> ServiceResult<SqlQueryListResponse> {
    let queries = load_saved_queries();
    Ok(SqlQueryListResponse { queries })
}

/// Save a query
pub fn query_save(request: SqlQuerySaveRequest) -> ServiceResult<SqlQuerySaveResponse> {
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
            return Err(ServiceError::NotFound(format!(
                "Query with id {} not found",
                id
            )));
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
pub fn query_delete(request: SqlQueryDeleteRequest) -> ServiceResult<SqlQueryDeleteResponse> {
    let mut queries = load_saved_queries();
    let initial_len = queries.len();
    queries.retain(|q| q.id != request.id);

    if queries.len() == initial_len {
        return Err(ServiceError::NotFound(format!(
            "Query with id {} not found",
            request.id
        )));
    }

    save_queries_to_file(&queries)?;

    Ok(SqlQueryDeleteResponse { success: true })
}
