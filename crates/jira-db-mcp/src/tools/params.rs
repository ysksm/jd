//! Tool parameter definitions

use serde::{Deserialize, Serialize};

/// Parameters for searching issues
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchIssuesParams {
    /// Search query text
    #[schemars(description = "Text to search for in issue summary and description")]
    pub query: Option<String>,

    /// Filter by project key
    #[schemars(description = "Project key to filter by (e.g., 'PROJ')")]
    pub project: Option<String>,

    /// Filter by status
    #[schemars(description = "Status to filter by (e.g., 'Open', 'In Progress', 'Done')")]
    pub status: Option<String>,

    /// Filter by assignee
    #[schemars(description = "Assignee name to filter by")]
    pub assignee: Option<String>,

    /// Maximum number of results to return
    #[schemars(description = "Maximum number of results (default: 20)")]
    pub limit: Option<usize>,

    /// Number of results to skip
    #[schemars(description = "Number of results to skip for pagination")]
    pub offset: Option<usize>,
}

/// Parameters for getting a specific issue
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetIssueParams {
    /// Issue key (e.g., "PROJ-123")
    #[schemars(description = "The issue key (e.g., 'PROJ-123')")]
    pub issue_key: String,

    /// Whether to include raw JSON data
    #[schemars(description = "Include raw JSON data from JIRA API")]
    pub include_raw: Option<bool>,
}

/// Parameters for getting issue change history
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetIssueHistoryParams {
    /// Issue key (e.g., "PROJ-123")
    #[schemars(description = "The issue key (e.g., 'PROJ-123')")]
    pub issue_key: String,

    /// Filter by field name
    #[schemars(description = "Field name to filter history by (e.g., 'status', 'assignee')")]
    pub field: Option<String>,

    /// Maximum number of results
    #[schemars(description = "Maximum number of history entries to return")]
    pub limit: Option<usize>,
}

/// Parameters for listing projects
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListProjectsParams {
    /// Filter to only show enabled projects
    #[schemars(description = "Only show projects enabled for sync")]
    pub enabled_only: Option<bool>,
}

/// Parameters for getting project metadata
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetProjectMetadataParams {
    /// Project key
    #[schemars(description = "Project key (e.g., 'PROJ')")]
    pub project_key: String,

    /// Metadata type to filter
    #[schemars(description = "Type of metadata: 'status', 'priority', 'issue-type', 'label', 'component', 'version', or 'all'")]
    pub metadata_type: Option<String>,
}

/// Parameters for getting database schema
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSchemaParams {
    /// Table name to get schema for
    #[schemars(description = "Table name to get schema for (optional, returns all tables if not specified)")]
    pub table: Option<String>,
}

/// Parameters for executing SQL queries
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExecuteSqlParams {
    /// SQL query (SELECT only)
    #[schemars(description = "SQL query to execute (SELECT statements only for read-only access)")]
    pub query: String,

    /// Maximum number of rows to return
    #[schemars(description = "Maximum number of rows to return (default: 100)")]
    pub limit: Option<usize>,
}

/// Parameters for semantic search
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SemanticSearchParams {
    /// Search query for semantic matching
    #[schemars(description = "Natural language query for semantic search")]
    pub query: String,

    /// Filter by project key
    #[schemars(description = "Project key to filter by")]
    pub project: Option<String>,

    /// Maximum number of results
    #[schemars(description = "Maximum number of results (default: 10)")]
    pub limit: Option<usize>,
}

/// Issue response for JSON output
#[derive(Debug, Serialize)]
pub struct IssueResponse {
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub issue_type: Option<String>,
    pub project_id: String,
    pub created_date: Option<String>,
    pub updated_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<String>,
}

impl From<jira_db_core::Issue> for IssueResponse {
    fn from(issue: jira_db_core::Issue) -> Self {
        Self {
            key: issue.key,
            summary: issue.summary,
            description: issue.description,
            status: issue.status,
            priority: issue.priority,
            assignee: issue.assignee,
            reporter: issue.reporter,
            issue_type: issue.issue_type,
            project_id: issue.project_id,
            created_date: issue.created_date.map(|d| d.to_rfc3339()),
            updated_date: issue.updated_date.map(|d| d.to_rfc3339()),
            raw_json: issue.raw_json,
        }
    }
}

/// Project response for JSON output
#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

impl From<jira_db_core::Project> for ProjectResponse {
    fn from(project: jira_db_core::Project) -> Self {
        Self {
            id: project.id,
            key: project.key,
            name: project.name,
            description: project.description,
        }
    }
}

/// Change history item response
#[derive(Debug, Serialize)]
pub struct ChangeHistoryResponse {
    pub issue_key: String,
    pub field: String,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub author: Option<String>,
    pub changed_at: String,
}

impl From<jira_db_core::ChangeHistoryItem> for ChangeHistoryResponse {
    fn from(item: jira_db_core::ChangeHistoryItem) -> Self {
        Self {
            issue_key: item.issue_key,
            field: item.field,
            from_value: item.from_string,
            to_value: item.to_string,
            author: item.author_display_name,
            changed_at: item.changed_at.to_rfc3339(),
        }
    }
}
