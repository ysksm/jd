//! Tool registry for managing MCP tools

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use jira_db_core::DatabaseFactory;
use serde_json::Value;

use super::implementations::*;
use crate::protocol::{CallToolResult, Tool, ToolInputSchema};

/// Trait for tool implementations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> Tool;

    /// Execute the tool with the given arguments
    async fn execute(&self, arguments: Value) -> Result<CallToolResult>;
}

/// Registry of available MCP tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolHandler>>,
}

impl ToolRegistry {
    /// Create a new tool registry with all available tools
    pub fn new(db_factory: Arc<DatabaseFactory>) -> Self {
        let mut tools: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();

        // Register all tools
        let search_issues = Arc::new(SearchIssuesTool::new(db_factory.clone()));
        tools.insert("search_issues".to_string(), search_issues);

        let get_issue = Arc::new(GetIssueTool::new(db_factory.clone()));
        tools.insert("get_issue".to_string(), get_issue);

        let get_issue_history = Arc::new(GetIssueHistoryTool::new(db_factory.clone()));
        tools.insert("get_issue_history".to_string(), get_issue_history);

        let list_projects = Arc::new(ListProjectsTool::new(db_factory.clone()));
        tools.insert("list_projects".to_string(), list_projects);

        let get_project_metadata = Arc::new(GetProjectMetadataTool::new(db_factory.clone()));
        tools.insert("get_project_metadata".to_string(), get_project_metadata);

        let get_schema = Arc::new(GetSchemaTool::new(db_factory.clone()));
        tools.insert("get_schema".to_string(), get_schema);

        let execute_sql = Arc::new(ExecuteSqlTool::new(db_factory.clone()));
        tools.insert("execute_sql".to_string(), execute_sql);

        let semantic_search = Arc::new(SemanticSearchTool::new(db_factory.clone()));
        tools.insert("semantic_search".to_string(), semantic_search);

        Self { tools }
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Call a tool by name
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        tool.execute(arguments).await
    }
}

/// Helper to build a Tool definition from a params struct
pub fn build_tool_definition<T: schemars::JsonSchema>(name: &str, description: &str) -> Tool {
    let schema = schemars::schema_for!(T);
    let schema_json = serde_json::to_value(schema).unwrap_or_default();

    Tool {
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: ToolInputSchema::from_json_schema(schema_json),
    }
}

#[cfg(test)]
mod tests {
    use super::super::params::SearchIssuesParams;
    use super::*;

    #[test]
    fn test_build_tool_definition() {
        let tool =
            build_tool_definition::<SearchIssuesParams>("search_issues", "Search for JIRA issues");

        assert_eq!(tool.name, "search_issues");
        assert_eq!(tool.description, Some("Search for JIRA issues".to_string()));
        assert_eq!(tool.input_schema.schema_type, "object");
    }
}
