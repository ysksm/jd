//! Tool implementations

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use duckdb::Connection;
use serde_json::Value;
use std::sync::Mutex;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, GetChangeHistoryUseCase, GetProjectMetadataUseCase, IssueRepository,
    ProjectRepository, SearchIssuesUseCase, SearchParams,
};

use crate::protocol::{CallToolResult, Tool};
use super::params::*;
use super::registry::{build_tool_definition, ToolHandler};

//=============================================================================
// SearchIssuesTool
//=============================================================================

pub struct SearchIssuesTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl SearchIssuesTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for SearchIssuesTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<SearchIssuesParams>(
            "search_issues",
            "Search for JIRA issues by text query, project, status, or assignee",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: SearchIssuesParams = serde_json::from_value(arguments)?;

        let repo = DuckDbIssueRepository::new(self.db_conn.clone());
        let use_case = SearchIssuesUseCase::new(Arc::new(repo));

        let search_params = SearchParams {
            query: params.query,
            project_key: params.project,
            status: params.status,
            assignee: params.assignee,
            limit: Some(params.limit.unwrap_or(20)),
            offset: params.offset,
        };

        let issues = use_case.execute(search_params)?;
        let response: Vec<IssueResponse> = issues.into_iter().map(Into::into).collect();
        let json = serde_json::to_string_pretty(&response)?;

        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetIssueTool
//=============================================================================

pub struct GetIssueTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl GetIssueTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for GetIssueTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetIssueParams>(
            "get_issue",
            "Get detailed information about a specific JIRA issue by its key",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetIssueParams = serde_json::from_value(arguments)?;

        let repo = DuckDbIssueRepository::new(self.db_conn.clone());

        let search_params = SearchParams {
            query: Some(params.issue_key.clone()),
            limit: Some(100),
            ..Default::default()
        };

        let issues = repo.search(&search_params)?;

        let issue = issues
            .into_iter()
            .find(|i| i.key == params.issue_key)
            .ok_or_else(|| anyhow::anyhow!("Issue {} not found", params.issue_key))?;

        let mut response: IssueResponse = issue.into();

        if !params.include_raw.unwrap_or(false) {
            response.raw_json = None;
        }

        let json = serde_json::to_string_pretty(&response)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetIssueHistoryTool
//=============================================================================

pub struct GetIssueHistoryTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl GetIssueHistoryTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for GetIssueHistoryTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetIssueHistoryParams>(
            "get_issue_history",
            "Get the change history for a JIRA issue, showing field changes over time",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetIssueHistoryParams = serde_json::from_value(arguments)?;

        let repo = DuckDbChangeHistoryRepository::new(self.db_conn.clone());
        let use_case = GetChangeHistoryUseCase::new(Arc::new(repo));

        let history = use_case.execute(&params.issue_key, params.field.as_deref())?;

        let response: Vec<ChangeHistoryResponse> = history
            .into_iter()
            .take(params.limit.unwrap_or(100))
            .map(Into::into)
            .collect();

        let json = serde_json::to_string_pretty(&response)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// ListProjectsTool
//=============================================================================

pub struct ListProjectsTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl ListProjectsTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for ListProjectsTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<ListProjectsParams>(
            "list_projects",
            "List all JIRA projects in the database",
        )
    }

    async fn execute(&self, _arguments: Value) -> Result<CallToolResult> {
        let repo = DuckDbProjectRepository::new(self.db_conn.clone());
        let projects = repo.find_all()?;

        let response: Vec<ProjectResponse> = projects.into_iter().map(Into::into).collect();
        let json = serde_json::to_string_pretty(&response)?;

        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetProjectMetadataTool
//=============================================================================

pub struct GetProjectMetadataTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl GetProjectMetadataTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for GetProjectMetadataTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetProjectMetadataParams>(
            "get_project_metadata",
            "Get metadata for a project including statuses, priorities, issue types, labels, components, and versions",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetProjectMetadataParams = serde_json::from_value(arguments)?;

        let project_repo = DuckDbProjectRepository::new(self.db_conn.clone());
        let project = project_repo
            .find_by_key(&params.project_key)?
            .ok_or_else(|| anyhow::anyhow!("Project {} not found", params.project_key))?;

        let metadata_repo = DuckDbMetadataRepository::new(self.db_conn.clone());
        let use_case = GetProjectMetadataUseCase::new(Arc::new(metadata_repo));

        let metadata = match &params.metadata_type {
            Some(mt) => use_case.execute_by_type(&project.id, mt)?,
            None => use_case.execute(&project.id)?,
        };

        let result = serde_json::json!({
            "project_key": params.project_key,
            "statuses": metadata.statuses.iter().map(|s| &s.name).collect::<Vec<_>>(),
            "priorities": metadata.priorities.iter().map(|p| &p.name).collect::<Vec<_>>(),
            "issue_types": metadata.issue_types.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "labels": metadata.labels.iter().map(|l| &l.name).collect::<Vec<_>>(),
            "components": metadata.components.iter().map(|c| &c.name).collect::<Vec<_>>(),
            "fix_versions": metadata.fix_versions.iter().map(|v| &v.name).collect::<Vec<_>>(),
        });

        let json = serde_json::to_string_pretty(&result)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetSchemaTool
//=============================================================================

pub struct GetSchemaTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl GetSchemaTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for GetSchemaTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetSchemaParams>(
            "get_schema",
            "Get the database schema, including table names and column definitions",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetSchemaParams = serde_json::from_value(arguments)?;

        let conn = self
            .db_conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock connection: {}", e))?;

        let query = match &params.table {
            Some(t) => format!(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
                t.replace('\'', "''")
            ),
            None => "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;

        let result = if params.table.is_some() {
            let rows = stmt.query_map([], |row| {
                Ok(serde_json::json!({
                    "column_name": row.get::<_, String>(0)?,
                    "data_type": row.get::<_, String>(1)?,
                    "is_nullable": row.get::<_, String>(2)?,
                }))
            })?;

            let columns: Vec<_> = rows.filter_map(|r| r.ok()).collect();

            serde_json::json!({
                "table": params.table,
                "columns": columns
            })
        } else {
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            let tables: Vec<_> = rows.filter_map(|r| r.ok()).collect();

            serde_json::json!({
                "tables": tables
            })
        };

        let json = serde_json::to_string_pretty(&result)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// ExecuteSqlTool
//=============================================================================

pub struct ExecuteSqlTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl ExecuteSqlTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for ExecuteSqlTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<ExecuteSqlParams>(
            "execute_sql",
            "Execute a read-only SQL query (SELECT statements only) on the JIRA database",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: ExecuteSqlParams = serde_json::from_value(arguments)?;

        // Security checks
        let query_upper = params.query.trim().to_uppercase();
        if !query_upper.starts_with("SELECT") {
            return Ok(CallToolResult::error(
                "Only SELECT queries are allowed for read-only access",
            ));
        }

        let dangerous_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
        ];
        for keyword in dangerous_keywords {
            if query_upper.contains(keyword) {
                return Ok(CallToolResult::error(format!(
                    "Query contains forbidden keyword: {}",
                    keyword
                )));
            }
        }

        let conn = self
            .db_conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock connection: {}", e))?;

        let final_query = if !query_upper.contains("LIMIT") {
            format!(
                "{} LIMIT {}",
                params.query.trim().trim_end_matches(';'),
                params.limit.unwrap_or(100)
            )
        } else {
            params.query.clone()
        };

        let mut stmt = conn.prepare(&final_query)?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                stmt.column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "?".to_string())
            })
            .collect();

        let rows_result = stmt.query_map([], |row| {
            let mut row_data = serde_json::Map::new();
            for (i, col_name) in column_names.iter().enumerate() {
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
                row_data.insert(col_name.clone(), value);
            }
            Ok(serde_json::Value::Object(row_data))
        })?;

        let rows: Vec<serde_json::Value> = rows_result.filter_map(|r| r.ok()).collect();

        let result = serde_json::json!({
            "columns": column_names,
            "rows": rows,
            "row_count": rows.len()
        });

        let json = serde_json::to_string_pretty(&result)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// SemanticSearchTool
//=============================================================================

pub struct SemanticSearchTool {
    db_conn: Arc<Mutex<Connection>>,
}

impl SemanticSearchTool {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self { db_conn }
    }
}

#[async_trait]
impl ToolHandler for SemanticSearchTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<SemanticSearchParams>(
            "semantic_search",
            "Search for issues using natural language semantic search (requires embeddings to be generated during sync)",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: SemanticSearchParams = serde_json::from_value(arguments)?;

        // Check if embeddings table exists and has data
        let embeddings_repo = jira_db_core::EmbeddingsRepository::new(self.db_conn.clone());

        // Check if we have embeddings
        let count = match embeddings_repo.count() {
            Ok(c) => c,
            Err(_) => {
                return Ok(CallToolResult::error(
                    "Semantic search is not available. Embeddings table not initialized. Please run 'jira-db sync' with embedding generation enabled.",
                ));
            }
        };

        if count == 0 {
            return Ok(CallToolResult::error(
                "No embeddings found. Please run 'jira-db sync' with embedding generation enabled to generate embeddings for issues.",
            ));
        }

        // For now, return info about embeddings
        // Full implementation requires OpenAI API key to embed the query
        let result = serde_json::json!({
            "status": "embeddings_available",
            "embedding_count": count,
            "message": "Semantic search requires an OpenAI API key to embed the query. This feature will be fully available when embedding configuration is added to the MCP server.",
            "query": params.query,
            "project_filter": params.project,
            "limit": params.limit.unwrap_or(10)
        });

        let json = serde_json::to_string_pretty(&result)?;
        Ok(CallToolResult::text(json))
    }
}
