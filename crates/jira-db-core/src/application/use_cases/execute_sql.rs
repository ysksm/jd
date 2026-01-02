//! Execute SQL query use case

use std::sync::{Arc, Mutex};

use duckdb::Connection;
use serde::{Deserialize, Serialize};

use crate::domain::error::{DomainError, DomainResult};

/// SQL execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
}

/// SQL execution use case
pub struct ExecuteSqlUseCase {
    db_conn: Arc<Mutex<Connection>>,
}

impl ExecuteSqlUseCase {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }

    /// Execute a read-only SQL query
    pub fn execute(&self, query: &str, limit: Option<usize>) -> DomainResult<SqlResult> {
        // Security checks - only allow SELECT queries
        let query_upper = query.trim().to_uppercase();
        if !query_upper.starts_with("SELECT") {
            return Err(DomainError::Validation(
                "Only SELECT queries are allowed for read-only access".to_string(),
            ));
        }

        let dangerous_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
        ];
        for keyword in dangerous_keywords {
            if query_upper.contains(keyword) {
                return Err(DomainError::Validation(format!(
                    "Query contains forbidden keyword: {}",
                    keyword
                )));
            }
        }

        let conn = self
            .db_conn
            .lock()
            .map_err(|e| DomainError::Repository(format!("Failed to lock connection: {}", e)))?;

        // Add LIMIT if not present
        let final_query = if !query_upper.contains("LIMIT") {
            format!(
                "{} LIMIT {}",
                query.trim().trim_end_matches(';'),
                limit.unwrap_or(100)
            )
        } else {
            query.to_string()
        };

        let mut stmt = conn
            .prepare(&final_query)
            .map_err(|e| DomainError::Repository(format!("SQL prepare error: {}", e)))?;

        // Execute query first - DuckDB requires this before column_count works
        let mut rows_iter = stmt
            .query([])
            .map_err(|e| DomainError::Repository(format!("Query execution error: {}", e)))?;

        // Collect raw row data first while we have mutable borrow
        let mut raw_rows: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut column_count = 0;

        while let Some(row) = rows_iter
            .next()
            .map_err(|e| DomainError::Repository(format!("Row iteration error: {}", e)))?
        {
            // Get column count from first row
            if column_count == 0 {
                column_count = row.as_ref().column_count();
            }

            let mut row_values: Vec<serde_json::Value> = Vec::new();
            for i in 0..column_count {
                let value: serde_json::Value = match row.get_ref(i) {
                    Ok(val) => match val {
                        duckdb::types::ValueRef::Null => serde_json::Value::Null,
                        duckdb::types::ValueRef::Boolean(b) => serde_json::Value::Bool(b),
                        duckdb::types::ValueRef::TinyInt(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::SmallInt(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::Int(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::BigInt(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::Float(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::Double(n) => serde_json::json!(n),
                        duckdb::types::ValueRef::Text(s) => {
                            serde_json::Value::String(String::from_utf8_lossy(s).to_string())
                        }
                        _ => serde_json::Value::String(format!("{:?}", val)),
                    },
                    Err(_) => serde_json::Value::Null,
                };
                row_values.push(value);
            }
            raw_rows.push(row_values);
        }

        // After dropping rows_iter, get column names from statement
        drop(rows_iter);

        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                stmt.column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| format!("col_{}", i))
            })
            .collect();

        let row_count = raw_rows.len();

        Ok(SqlResult {
            columns: column_names,
            rows: raw_rows,
            row_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    fn create_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

        // Create test table
        conn.execute(
            "CREATE TABLE test_table (id INTEGER, name VARCHAR, value DOUBLE)",
            [],
        )
        .expect("Failed to create table");

        // Insert test data
        conn.execute(
            "INSERT INTO test_table VALUES (1, 'Alice', 10.5), (2, 'Bob', 20.0), (3, 'Charlie', 30.5)",
            [],
        )
        .expect("Failed to insert data");

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_execute_select_query() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        let result = use_case
            .execute("SELECT * FROM test_table", None)
            .expect("Query should succeed");

        assert_eq!(result.columns, vec!["id", "name", "value"]);
        assert_eq!(result.row_count, 3);
        assert_eq!(result.rows.len(), 3);

        // Check first row
        assert_eq!(result.rows[0][0], serde_json::json!(1));
        assert_eq!(result.rows[0][1], serde_json::json!("Alice"));
    }

    #[test]
    fn test_execute_with_limit() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        let result = use_case
            .execute("SELECT * FROM test_table", Some(2))
            .expect("Query should succeed");

        assert_eq!(result.row_count, 2);
    }

    #[test]
    fn test_reject_non_select_query() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        let result = use_case.execute("INSERT INTO test_table VALUES (4, 'Dave', 40.0)", None);

        assert!(result.is_err());
        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("Only SELECT queries"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_reject_dangerous_keywords() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        let result = use_case.execute("SELECT * FROM test_table; DROP TABLE test_table", None);

        assert!(result.is_err());
        match result {
            Err(DomainError::Validation(msg)) => {
                assert!(msg.contains("forbidden keyword"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_execute_with_where_clause() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        let result = use_case
            .execute("SELECT name FROM test_table WHERE id = 2", None)
            .expect("Query should succeed");

        assert_eq!(result.columns, vec!["name"]);
        assert_eq!(result.row_count, 1);
        assert_eq!(result.rows[0][0], serde_json::json!("Bob"));
    }
}
