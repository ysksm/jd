//! MCP Server implementation for JIRA database

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::handler::server::tool::Parameters;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, Implementation, ProtocolVersion, ServerCapabilities,
    ServerInfo,
};
use rmcp::tool;
use rmcp::{ErrorData as McpError, ServerHandler};

use duckdb::Connection;
use std::sync::Mutex;

use jira_db_core::{
    Database, DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, GetChangeHistoryUseCase, GetProjectMetadataUseCase, IssueRepository,
    ProjectRepository, SearchIssuesUseCase, SearchParams,
};

use crate::config::McpConfig;
use crate::tools::*;

/// JIRA Database MCP Service
#[derive(Clone)]
pub struct JiraDbService {
    tool_router: ToolRouter<Self>,
    db_conn: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    config: McpConfig,
}

impl JiraDbService {
    /// Create a new JiraDbService instance
    pub fn new(config: McpConfig) -> Result<Self, anyhow::Error> {
        let db = Database::new(&config.database_path)?;

        Ok(Self {
            tool_router: Self::tool_router(),
            db_conn: db.connection(),
            config,
        })
    }

    fn create_mcp_error(message: impl Into<String>) -> McpError {
        McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::Owned(message.into()),
            data: None,
        }
    }
}

/// Tool implementations
#[rmcp::tool_router]
impl JiraDbService {
    /// Search issues in the JIRA database
    #[tool(description = "Search for JIRA issues by text query, project, status, or assignee")]
    async fn search_issues(
        &self,
        Parameters(params): Parameters<SearchIssuesParams>,
    ) -> Result<CallToolResult, McpError> {
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

        let issues = use_case
            .execute(search_params)
            .map_err(|e| Self::create_mcp_error(format!("Search failed: {}", e)))?;

        let response: Vec<IssueResponse> = issues.into_iter().map(Into::into).collect();
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get a specific issue by key
    #[tool(description = "Get detailed information about a specific JIRA issue by its key")]
    async fn get_issue(
        &self,
        Parameters(params): Parameters<GetIssueParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = DuckDbIssueRepository::new(self.db_conn.clone());

        let search_params = SearchParams {
            query: Some(params.issue_key.clone()),
            limit: Some(100),
            ..Default::default()
        };

        let issues = repo
            .search(&search_params)
            .map_err(|e| Self::create_mcp_error(format!("Failed to get issue: {}", e)))?;

        let issue = issues
            .into_iter()
            .find(|i| i.key == params.issue_key)
            .ok_or_else(|| Self::create_mcp_error(format!("Issue {} not found", params.issue_key)))?;

        let mut response: IssueResponse = issue.into();

        if !params.include_raw.unwrap_or(false) {
            response.raw_json = None;
        }

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get change history for an issue
    #[tool(description = "Get the change history for a JIRA issue, showing field changes over time")]
    async fn get_issue_history(
        &self,
        Parameters(params): Parameters<GetIssueHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = DuckDbChangeHistoryRepository::new(self.db_conn.clone());
        let use_case = GetChangeHistoryUseCase::new(Arc::new(repo));

        let history = use_case
            .execute(&params.issue_key, params.field.as_deref())
            .map_err(|e| Self::create_mcp_error(format!("Failed to get history: {}", e)))?;

        let response: Vec<ChangeHistoryResponse> = history
            .into_iter()
            .take(params.limit.unwrap_or(100))
            .map(Into::into)
            .collect();

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List all projects
    #[tool(description = "List all JIRA projects in the database")]
    async fn list_projects(
        &self,
        Parameters(_params): Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = DuckDbProjectRepository::new(self.db_conn.clone());

        let projects = repo
            .find_all()
            .map_err(|e| Self::create_mcp_error(format!("Failed to list projects: {}", e)))?;

        let response: Vec<ProjectResponse> = projects.into_iter().map(Into::into).collect();

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get project metadata
    #[tool(description = "Get metadata for a project including statuses, priorities, issue types, labels, components, and versions")]
    async fn get_project_metadata(
        &self,
        Parameters(params): Parameters<GetProjectMetadataParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_repo = DuckDbProjectRepository::new(self.db_conn.clone());
        let project = project_repo
            .find_by_key(&params.project_key)
            .map_err(|e| Self::create_mcp_error(format!("Failed to find project: {}", e)))?
            .ok_or_else(|| {
                Self::create_mcp_error(format!("Project {} not found", params.project_key))
            })?;

        let metadata_repo = DuckDbMetadataRepository::new(self.db_conn.clone());
        let use_case = GetProjectMetadataUseCase::new(Arc::new(metadata_repo));

        let metadata = match &params.metadata_type {
            Some(mt) => use_case
                .execute_by_type(&project.id, mt)
                .map_err(|e| Self::create_mcp_error(format!("Failed to get metadata: {}", e)))?,
            None => use_case
                .execute(&project.id)
                .map_err(|e| Self::create_mcp_error(format!("Failed to get metadata: {}", e)))?,
        };

        let json = serde_json::to_string_pretty(&serde_json::json!({
            "project_key": params.project_key,
            "statuses": metadata.statuses.iter().map(|s| &s.name).collect::<Vec<_>>(),
            "priorities": metadata.priorities.iter().map(|p| &p.name).collect::<Vec<_>>(),
            "issue_types": metadata.issue_types.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "labels": metadata.labels.iter().map(|l| &l.name).collect::<Vec<_>>(),
            "components": metadata.components.iter().map(|c| &c.name).collect::<Vec<_>>(),
            "fix_versions": metadata.fix_versions.iter().map(|v| &v.name).collect::<Vec<_>>(),
        }))
        .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get database schema information
    #[tool(description = "Get the database schema, including table names and column definitions")]
    async fn get_schema(
        &self,
        Parameters(params): Parameters<GetSchemaParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self
            .db_conn
            .lock()
            .map_err(|e| Self::create_mcp_error(format!("Failed to lock connection: {}", e)))?;

        let query = match &params.table {
            Some(t) => format!(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
                t.replace('\'', "''")
            ),
            None => "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name".to_string(),
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| Self::create_mcp_error(format!("Failed to prepare query: {}", e)))?;

        let result = if params.table.is_some() {
            let rows = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "column_name": row.get::<_, String>(0)?,
                        "data_type": row.get::<_, String>(1)?,
                        "is_nullable": row.get::<_, String>(2)?,
                    }))
                })
                .map_err(|e| Self::create_mcp_error(format!("Query failed: {}", e)))?;

            let columns: Vec<_> = rows.filter_map(|r| r.ok()).collect();

            serde_json::json!({
                "table": params.table,
                "columns": columns
            })
        } else {
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| Self::create_mcp_error(format!("Query failed: {}", e)))?;

            let tables: Vec<_> = rows.filter_map(|r| r.ok()).collect();

            serde_json::json!({
                "tables": tables
            })
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Execute a SQL query (SELECT only)
    #[tool(description = "Execute a read-only SQL query (SELECT statements only) on the JIRA database")]
    async fn execute_sql(
        &self,
        Parameters(params): Parameters<ExecuteSqlParams>,
    ) -> Result<CallToolResult, McpError> {
        let query_upper = params.query.trim().to_uppercase();
        if !query_upper.starts_with("SELECT") {
            return Err(Self::create_mcp_error(
                "Only SELECT queries are allowed for read-only access",
            ));
        }

        let dangerous_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
        ];
        for keyword in dangerous_keywords {
            if query_upper.contains(keyword) {
                return Err(Self::create_mcp_error(format!(
                    "Query contains forbidden keyword: {}",
                    keyword
                )));
            }
        }

        let conn = self
            .db_conn
            .lock()
            .map_err(|e| Self::create_mcp_error(format!("Failed to lock connection: {}", e)))?;

        let final_query = if !query_upper.contains("LIMIT") {
            format!(
                "{} LIMIT {}",
                params.query.trim().trim_end_matches(';'),
                params.limit.unwrap_or(100)
            )
        } else {
            params.query.clone()
        };

        let mut stmt = conn
            .prepare(&final_query)
            .map_err(|e| Self::create_mcp_error(format!("Failed to prepare query: {}", e)))?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                stmt.column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "?".to_string())
            })
            .collect();

        let rows_result = stmt
            .query_map([], |row| {
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
            })
            .map_err(|e| Self::create_mcp_error(format!("Query execution failed: {}", e)))?;

        let rows: Vec<serde_json::Value> = rows_result.filter_map(|r| r.ok()).collect();

        let result = serde_json::json!({
            "columns": column_names,
            "rows": rows,
            "row_count": rows.len()
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| Self::create_mcp_error(format!("JSON serialization failed: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Semantic search using vector embeddings
    #[tool(description = "Search for issues using natural language semantic search (requires embeddings to be generated)")]
    async fn semantic_search(
        &self,
        Parameters(params): Parameters<SemanticSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = params;
        Err(Self::create_mcp_error(
            "Semantic search is not yet configured. Please run 'jira-db sync' with embedding generation enabled.",
        ))
    }
}

/// Server handler implementation
#[rmcp::tool_handler]
impl ServerHandler for JiraDbService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "JIRA Database MCP Server - Query and search JIRA issues stored in a local DuckDB database. \
                 Available tools: search_issues, get_issue, get_issue_history, list_projects, \
                 get_project_metadata, get_schema, execute_sql, semantic_search."
                    .to_string(),
            ),
        }
    }
}
