use crate::application::dto::CreatedIssueDto;
use crate::domain::entities::{
    Component, FixVersion, Issue, IssueType, JiraField, Label, Priority, Project, Status,
};
use crate::domain::error::DomainResult;
use async_trait::async_trait;

/// Service trait for JIRA API operations
/// Infrastructure layer will implement this trait (DIP)
#[async_trait]
pub trait JiraService: Send + Sync {
    /// Fetch all projects from JIRA
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>>;

    /// Fetch all issues for a project
    async fn fetch_project_issues(&self, project_key: &str) -> DomainResult<Vec<Issue>>;

    /// Test connection to JIRA
    async fn test_connection(&self) -> DomainResult<()>;

    /// Fetch project statuses
    async fn fetch_project_statuses(&self, project_key: &str) -> DomainResult<Vec<Status>>;

    /// Fetch priorities
    async fn fetch_priorities(&self) -> DomainResult<Vec<Priority>>;

    /// Fetch project issue types
    async fn fetch_project_issue_types(&self, project_id: &str) -> DomainResult<Vec<IssueType>>;

    /// Fetch project labels
    async fn fetch_project_labels(&self, project_key: &str) -> DomainResult<Vec<Label>>;

    /// Fetch project components
    async fn fetch_project_components(&self, project_key: &str) -> DomainResult<Vec<Component>>;

    /// Fetch project versions
    async fn fetch_project_versions(&self, project_key: &str) -> DomainResult<Vec<FixVersion>>;

    /// Fetch all JIRA fields metadata
    async fn fetch_fields(&self) -> DomainResult<Vec<JiraField>>;

    /// Create a new issue
    async fn create_issue(
        &self,
        project_key: &str,
        summary: &str,
        description: Option<&str>,
        issue_type: &str,
    ) -> DomainResult<CreatedIssueDto>;
}
