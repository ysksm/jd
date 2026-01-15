//! Tool implementations

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use jira_db_core::{
    DatabaseFactory, DuckDbChangeHistoryRepository, DuckDbIssueRepository,
    DuckDbMetadataRepository, DuckDbProjectRepository, GetChangeHistoryUseCase,
    GetProjectMetadataUseCase, IssueRepository, ProjectRepository, RawDataRepository,
    SearchIssuesUseCase, SearchParams,
};
use serde_json::Value;

use super::params::*;
use super::registry::{ToolHandler, build_tool_definition};
use crate::protocol::{CallToolResult, Tool};

/// Helper function to extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
fn extract_project_key(issue_key: &str) -> Option<&str> {
    issue_key.split('-').next()
}

//=============================================================================
// SearchIssuesTool
//=============================================================================

pub struct SearchIssuesTool {
    db_factory: Arc<DatabaseFactory>,
}

impl SearchIssuesTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
    }
}

#[async_trait]
impl ToolHandler for SearchIssuesTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<SearchIssuesParams>(
            "search_issues",
            "Search for JIRA issues by text query, project, status, or assignee. Project key is required to specify which database to search.",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: SearchIssuesParams = serde_json::from_value(arguments)?;

        // Project key is required for per-project databases
        let project_key = match &params.project {
            Some(key) => key.clone(),
            None => {
                // If no project specified, search across all available databases
                let projects = self.db_factory.list_project_databases()?;
                if projects.is_empty() {
                    return Ok(CallToolResult::error(
                        "No project databases found. Run 'jira-db sync' first.",
                    ));
                }

                let mut all_issues = Vec::new();
                for project in &projects {
                    if let Ok(conn) = self.db_factory.get_connection(project) {
                        let repo = DuckDbIssueRepository::new(conn);
                        let use_case = SearchIssuesUseCase::new(Arc::new(repo));

                        let search_params = SearchParams {
                            query: params.query.clone(),
                            project_key: Some(project.clone()),
                            status: params.status.clone(),
                            assignee: params.assignee.clone(),
                            issue_type: None,
                            priority: None,
                            team: None,
                            limit: Some(params.limit.unwrap_or(20)),
                            offset: params.offset,
                        };

                        if let Ok(issues) = use_case.execute(search_params) {
                            all_issues.extend(issues);
                        }
                    }
                }

                let response: Vec<IssueResponse> = all_issues.into_iter().map(Into::into).collect();
                let json = serde_json::to_string_pretty(&response)?;
                return Ok(CallToolResult::text(json));
            }
        };

        let conn = self.db_factory.get_connection(&project_key)?;
        let repo = DuckDbIssueRepository::new(conn);
        let use_case = SearchIssuesUseCase::new(Arc::new(repo));

        let search_params = SearchParams {
            query: params.query,
            project_key: Some(project_key),
            status: params.status,
            assignee: params.assignee,
            issue_type: None,
            priority: None,
            team: None,
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
    db_factory: Arc<DatabaseFactory>,
}

impl GetIssueTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
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

        // Extract project key from issue key
        let project_key = extract_project_key(&params.issue_key)
            .ok_or_else(|| anyhow::anyhow!("Invalid issue key format: {}", params.issue_key))?;

        let conn = self.db_factory.get_connection(project_key)?;
        let repo = DuckDbIssueRepository::new(conn);

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
    db_factory: Arc<DatabaseFactory>,
}

impl GetIssueHistoryTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
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

        // Extract project key from issue key
        let project_key = extract_project_key(&params.issue_key)
            .ok_or_else(|| anyhow::anyhow!("Invalid issue key format: {}", params.issue_key))?;

        let conn = self.db_factory.get_connection(project_key)?;
        let repo = DuckDbChangeHistoryRepository::new(conn);
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
    db_factory: Arc<DatabaseFactory>,
}

impl ListProjectsTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
    }
}

#[async_trait]
impl ToolHandler for ListProjectsTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<ListProjectsParams>(
            "list_projects",
            "List all JIRA projects with databases available",
        )
    }

    async fn execute(&self, _arguments: Value) -> Result<CallToolResult> {
        // List all project databases
        let project_keys = self.db_factory.list_project_databases()?;

        let mut all_projects = Vec::new();
        for key in &project_keys {
            if let Ok(conn) = self.db_factory.get_connection(key) {
                let repo = DuckDbProjectRepository::new(conn);
                if let Ok(projects) = repo.find_all() {
                    for project in projects {
                        all_projects.push(ProjectResponse::from(project));
                    }
                }
            }
        }

        // If no projects found in databases, at least return the database keys
        if all_projects.is_empty() && !project_keys.is_empty() {
            let response = serde_json::json!({
                "available_databases": project_keys,
                "message": "Project databases found but no project metadata. Run 'jira-db sync' to populate."
            });
            let json = serde_json::to_string_pretty(&response)?;
            return Ok(CallToolResult::text(json));
        }

        let json = serde_json::to_string_pretty(&all_projects)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetProjectMetadataTool
//=============================================================================

pub struct GetProjectMetadataTool {
    db_factory: Arc<DatabaseFactory>,
}

impl GetProjectMetadataTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
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

        let conn = self.db_factory.get_connection(&params.project_key)?;

        let project_repo = DuckDbProjectRepository::new(conn.clone());
        let project = project_repo
            .find_by_key(&params.project_key)?
            .ok_or_else(|| anyhow::anyhow!("Project {} not found", params.project_key))?;

        let metadata_repo = DuckDbMetadataRepository::new(conn);
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
    db_factory: Arc<DatabaseFactory>,
}

impl GetSchemaTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
    }
}

#[async_trait]
impl ToolHandler for GetSchemaTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetSchemaParams>(
            "get_schema",
            "Get the database schema, including table names and column definitions. Requires project key to specify which database to query.",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetSchemaParams = serde_json::from_value(arguments)?;

        // Get project key (required for per-project databases)
        let project_key = match &params.project {
            Some(key) => key.clone(),
            None => {
                // Return list of available databases
                let projects = self.db_factory.list_project_databases()?;
                let result = serde_json::json!({
                    "available_databases": projects,
                    "message": "Please specify a project key using the 'project' parameter to get schema"
                });
                let json = serde_json::to_string_pretty(&result)?;
                return Ok(CallToolResult::text(json));
            }
        };

        let conn = self.db_factory.get_connection(&project_key)?;
        let conn = conn
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
                "project": project_key,
                "table": params.table,
                "columns": columns
            })
        } else {
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            let tables: Vec<_> = rows.filter_map(|r| r.ok()).collect();

            serde_json::json!({
                "project": project_key,
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
    db_factory: Arc<DatabaseFactory>,
}

impl ExecuteSqlTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
    }
}

#[async_trait]
impl ToolHandler for ExecuteSqlTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<ExecuteSqlParams>(
            "execute_sql",
            "Execute a read-only SQL query (SELECT statements only) on the JIRA database. Requires project key to specify which database to query.",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: ExecuteSqlParams = serde_json::from_value(arguments)?;

        // Get project key (required for per-project databases)
        let project_key = match &params.project {
            Some(key) => key.clone(),
            None => {
                // Return list of available databases
                let projects = self.db_factory.list_project_databases()?;
                return Ok(CallToolResult::error(format!(
                    "Project key required. Available databases: {:?}",
                    projects
                )));
            }
        };

        // Security checks - allow SELECT and WITH...SELECT (CTEs)
        // Skip comment lines (-- ...) to find the actual SQL statement
        let query_upper = params
            .query
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with("--") && !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();

        let is_select = query_upper.starts_with("SELECT");
        let is_with_select = query_upper.starts_with("WITH") && query_upper.contains("SELECT");

        if !is_select && !is_with_select {
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

        let conn = self.db_factory.get_connection(&project_key)?;
        let conn = conn
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
            "project": project_key,
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
    db_factory: Arc<DatabaseFactory>,
    openai_api_key: Option<String>,
}

impl SemanticSearchTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        // Try to get OpenAI API key from environment
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        Self {
            db_factory,
            openai_api_key,
        }
    }

    #[allow(dead_code)]
    pub fn with_api_key(db_factory: Arc<DatabaseFactory>, api_key: Option<String>) -> Self {
        let openai_api_key = api_key.or_else(|| std::env::var("OPENAI_API_KEY").ok());
        Self {
            db_factory,
            openai_api_key,
        }
    }
}

#[async_trait]
impl ToolHandler for SemanticSearchTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<SemanticSearchParams>(
            "semantic_search",
            "Search for issues using natural language semantic search (requires embeddings to be generated with 'jira-db embeddings'). Project key is required.",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: SemanticSearchParams = serde_json::from_value(arguments)?;

        // Project key is required
        let project_key = match &params.project {
            Some(key) => key.clone(),
            None => {
                return Ok(CallToolResult::error(
                    "Project key is required for semantic search. Use the 'project' parameter.",
                ));
            }
        };

        let conn = self.db_factory.get_connection(&project_key)?;

        // Check if embeddings table exists and has data
        let embeddings_repo = jira_db_core::EmbeddingsRepository::new(conn);

        // Check if we have embeddings
        let count = match embeddings_repo.count() {
            Ok(c) => c,
            Err(_) => {
                return Ok(CallToolResult::error(
                    "Semantic search is not available. Embeddings table not initialized. Please run 'jira-db embeddings' to generate embeddings for issues.",
                ));
            }
        };

        if count == 0 {
            return Ok(CallToolResult::error(
                "No embeddings found. Please run 'jira-db embeddings' to generate embeddings for issues.",
            ));
        }

        // Check if we have an API key for embedding the query
        let api_key = match &self.openai_api_key {
            Some(key) => key.clone(),
            None => {
                return Ok(CallToolResult::error(
                    "Semantic search requires OPENAI_API_KEY environment variable to be set for query embedding.",
                ));
            }
        };

        // Create embedding client for the query
        let embedding_config = jira_db_core::EmbeddingConfig::new(api_key);
        let embedding_client = match jira_db_core::OpenAIEmbeddingClient::new(embedding_config) {
            Ok(client) => client,
            Err(e) => {
                return Ok(CallToolResult::error(format!(
                    "Failed to create embedding client: {}",
                    e
                )));
            }
        };

        // Embed the query
        use jira_db_core::EmbeddingProvider;
        let query_embedding = match embedding_client.embed(&params.query).await {
            Ok(embedding) => embedding,
            Err(e) => {
                return Ok(CallToolResult::error(format!(
                    "Failed to embed query: {}",
                    e
                )));
            }
        };

        // Perform semantic search
        let limit = params.limit.unwrap_or(10);
        let results =
            match embeddings_repo.semantic_search(&query_embedding, Some(&project_key), limit) {
                Ok(results) => results,
                Err(e) => {
                    return Ok(CallToolResult::error(format!(
                        "Failed to perform semantic search: {}",
                        e
                    )));
                }
            };

        // Format results
        let response: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "issue_key": r.issue_key,
                    "summary": r.summary,
                    "description": r.description,
                    "status": r.status,
                    "project_id": r.project_id,
                    "similarity_score": r.similarity_score
                })
            })
            .collect();

        let result = serde_json::json!({
            "project": project_key,
            "query": params.query,
            "result_count": response.len(),
            "results": response
        });

        let json = serde_json::to_string_pretty(&result)?;
        Ok(CallToolResult::text(json))
    }
}

//=============================================================================
// GetRawIssueDataTool
//=============================================================================

pub struct GetRawIssueDataTool {
    db_factory: Arc<DatabaseFactory>,
}

impl GetRawIssueDataTool {
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        Self { db_factory }
    }
}

#[async_trait]
impl ToolHandler for GetRawIssueDataTool {
    fn definition(&self) -> Tool {
        build_tool_definition::<GetRawIssueDataParams>(
            "get_raw_issue_data",
            "Get the raw JSON data from JIRA API for a specific issue. This data is stored separately from processed issue data.",
        )
    }

    async fn execute(&self, arguments: Value) -> Result<CallToolResult> {
        let params: GetRawIssueDataParams = serde_json::from_value(arguments)?;

        // Extract project key from issue key
        let project_key = extract_project_key(&params.issue_key)
            .ok_or_else(|| anyhow::anyhow!("Invalid issue key format: {}", params.issue_key))?;

        // Get raw connection for this project
        let raw_conn = match self.db_factory.get_raw_connection(project_key) {
            Ok(conn) => conn,
            Err(_) => {
                return Ok(CallToolResult::error(format!(
                    "Raw data database not found for project {}. Run sync with raw data enabled.",
                    project_key
                )));
            }
        };

        let raw_repo = RawDataRepository::new(raw_conn);

        match raw_repo.get_issue_raw_data(&params.issue_key)? {
            Some(raw_data) => {
                // Parse and re-format the JSON for pretty output
                let parsed: serde_json::Value = serde_json::from_str(&raw_data)
                    .unwrap_or_else(|_| serde_json::Value::String(raw_data));
                let json = serde_json::to_string_pretty(&parsed)?;
                Ok(CallToolResult::text(json))
            }
            None => Ok(CallToolResult::error(format!(
                "Raw data not found for issue {}. It may not have been synced with raw data enabled.",
                params.issue_key
            ))),
        }
    }
}
