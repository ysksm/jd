//! Execute SQL query use case

use std::sync::{Arc, Mutex};

use duckdb::Connection;
use serde::{Deserialize, Serialize};

use crate::domain::error::{DomainError, DomainResult};

/// Check if a keyword appears as a whole word in the text
/// Returns true if the keyword is found and not part of a larger word
fn is_whole_word(text: &str, keyword: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(keyword) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0
            || !text
                .chars()
                .nth(abs_pos - 1)
                .unwrap_or(' ')
                .is_alphanumeric();
        let after_pos = abs_pos + keyword.len();
        let after_ok = after_pos >= text.len()
            || !text.chars().nth(after_pos).unwrap_or(' ').is_alphanumeric();

        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
    }
    false
}

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

        // Check for dangerous SQL keywords (as whole words, not substrings)
        // This allows "updated_at" while blocking "UPDATE"
        let dangerous_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
        ];
        for keyword in dangerous_keywords {
            // Check if keyword appears as a whole word (not part of another word)
            if is_whole_word(&query_upper, keyword) {
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

        // Execute query first with query_map, collecting rows with dynamic column detection
        let mut detected_column_count: Option<usize> = None;
        let rows_result = stmt
            .query_map([], |row| {
                // Detect column count from first row
                let column_count = row.as_ref().column_count();

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
                            duckdb::types::ValueRef::HugeInt(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::UTinyInt(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::USmallInt(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::UInt(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::UBigInt(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::Float(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::Double(n) => serde_json::json!(n),
                            duckdb::types::ValueRef::Text(s) => {
                                serde_json::Value::String(String::from_utf8_lossy(s).to_string())
                            }
                            duckdb::types::ValueRef::Blob(b) => {
                                serde_json::Value::String(format!("<blob:{} bytes>", b.len()))
                            }
                            duckdb::types::ValueRef::Timestamp(unit, val) => {
                                // Convert timestamp to string representation
                                let ts_str = match unit {
                                    duckdb::types::TimeUnit::Second => {
                                        chrono::DateTime::from_timestamp(val, 0)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_else(|| format!("{}s", val))
                                    }
                                    duckdb::types::TimeUnit::Millisecond => {
                                        chrono::DateTime::from_timestamp_millis(val)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_else(|| format!("{}ms", val))
                                    }
                                    duckdb::types::TimeUnit::Microsecond => {
                                        chrono::DateTime::from_timestamp_micros(val)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_else(|| format!("{}us", val))
                                    }
                                    duckdb::types::TimeUnit::Nanosecond => {
                                        chrono::DateTime::from_timestamp_nanos(val).to_rfc3339()
                                    }
                                };
                                serde_json::Value::String(ts_str)
                            }
                            duckdb::types::ValueRef::Date32(days) => {
                                // Days since Unix epoch
                                let date =
                                    chrono::NaiveDate::from_num_days_from_ce_opt(days + 719163)
                                        .map(|d| d.to_string())
                                        .unwrap_or_else(|| format!("date:{}", days));
                                serde_json::Value::String(date)
                            }
                            duckdb::types::ValueRef::Time64(unit, val) => {
                                serde_json::Value::String(format!("time:{:?}:{}", unit, val))
                            }
                            _ => serde_json::Value::String(format!("{:?}", val)),
                        },
                        Err(_) => serde_json::Value::Null,
                    };
                    row_values.push(value);
                }
                Ok((column_count, row_values))
            })
            .map_err(|e| DomainError::Repository(format!("Query execution error: {}", e)))?;

        let mut raw_rows: Vec<Vec<serde_json::Value>> = Vec::new();
        for result in rows_result {
            if let Ok((col_count, row_values)) = result {
                if detected_column_count.is_none() {
                    detected_column_count = Some(col_count);
                }
                raw_rows.push(row_values);
            }
        }
        let row_count = raw_rows.len();

        // Get column names from statement (now query has been executed)
        let column_count = detected_column_count.unwrap_or_else(|| stmt.column_count());
        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                stmt.column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| format!("col_{}", i))
            })
            .collect();

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

    #[test]
    fn test_execute_with_zero_rows_returns_columns() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        // Query that returns no rows
        let result = use_case
            .execute(
                "SELECT id, name, value FROM test_table WHERE id = 999",
                None,
            )
            .expect("Query should succeed");

        // Should still have column names even with 0 rows
        assert_eq!(result.columns, vec!["id", "name", "value"]);
        assert_eq!(result.row_count, 0);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn test_is_whole_word() {
        // Should match whole words
        assert!(is_whole_word("UPDATE table SET", "UPDATE"));
        assert!(is_whole_word("SELECT * FROM t; UPDATE", "UPDATE"));
        assert!(is_whole_word("DELETE FROM", "DELETE"));

        // Should NOT match substrings within words
        assert!(!is_whole_word("SELECT updated_at FROM", "UPDATE"));
        assert!(!is_whole_word(
            "SELECT created_at, deleted_flag FROM",
            "DELETE"
        ));
        assert!(!is_whole_word("SELECT execution_time FROM", "EXEC"));
    }

    #[test]
    fn test_allow_column_names_with_keywords() {
        let db = create_test_db();
        let use_case = ExecuteSqlUseCase::new(db);

        // Should allow columns like updated_at, created_at
        let result = use_case.execute("SELECT name AS updated_at FROM test_table", None);
        assert!(
            result.is_ok(),
            "Query with 'updated_at' column should be allowed"
        );
    }
}
