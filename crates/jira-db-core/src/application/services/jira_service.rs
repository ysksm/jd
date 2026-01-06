use crate::application::dto::{CreatedIssueDto, TransitionDto};
use crate::domain::entities::{
    Component, FixVersion, Issue, IssueType, JiraField, Label, Priority, Project, Status,
};
use crate::domain::error::DomainResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Progress information for resumable fetch
#[derive(Debug, Clone)]
pub struct FetchProgress {
    /// Fetched issues in this batch
    pub issues: Vec<Issue>,
    /// Total number of issues matching the query (may be 0 if API doesn't provide it)
    pub total: usize,
    /// Number of issues fetched so far (including this batch)
    pub fetched_so_far: usize,
    /// Whether there are more issues to fetch
    pub has_more: bool,
    /// Token for fetching the next page (for JIRA Cloud API)
    pub next_page_token: Option<String>,
}

/// Service trait for JIRA API operations
/// Infrastructure layer will implement this trait (DIP)
#[async_trait]
pub trait JiraService: Send + Sync {
    /// Fetch all projects from JIRA
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>>;

    /// Fetch all issues for a project
    async fn fetch_project_issues(&self, project_key: &str) -> DomainResult<Vec<Issue>>;

    /// Fetch a batch of issues for a project with resumable support
    /// Issues are fetched ordered by `updated ASC` (oldest first)
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `after_updated_at` - Only fetch issues updated at or after this timestamp (for resume)
    /// * `page_token` - Token for fetching the next page (None for first page)
    /// * `max_results` - Maximum number of issues to fetch in this batch
    ///
    /// # Returns
    /// FetchProgress containing the batch of issues and pagination info
    async fn fetch_project_issues_batch(
        &self,
        project_key: &str,
        after_updated_at: Option<DateTime<Utc>>,
        page_token: Option<&str>,
        max_results: usize,
    ) -> DomainResult<FetchProgress>;

    /// Test connection to JIRA
    async fn test_connection(&self) -> DomainResult<()>;

    /// Fetch project statuses
    async fn fetch_project_statuses(&self, project_key: &str) -> DomainResult<Vec<Status>>;

    /// Fetch priorities
    async fn fetch_priorities(&self) -> DomainResult<Vec<Priority>>;

    /// Fetch project issue types by project ID
    async fn fetch_project_issue_types(&self, project_id: &str) -> DomainResult<Vec<IssueType>>;

    /// Fetch project issue types by project key (for create issue)
    async fn fetch_issue_types_by_project_key(
        &self,
        project_key: &str,
    ) -> DomainResult<Vec<IssueType>>;

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

    /// Get available transitions for an issue
    async fn get_issue_transitions(&self, issue_key: &str) -> DomainResult<Vec<TransitionDto>>;

    /// Transition an issue to a new status
    async fn transition_issue(&self, issue_key: &str, transition_id: &str) -> DomainResult<()>;

    /// Get issue count by status for a project (for integrity check)
    async fn get_issue_count_by_status(
        &self,
        project_key: &str,
    ) -> DomainResult<std::collections::HashMap<String, usize>>;

    /// Get total issue count for a project (simple JQL count)
    async fn get_total_issue_count(&self, project_key: &str) -> DomainResult<usize>;
}
