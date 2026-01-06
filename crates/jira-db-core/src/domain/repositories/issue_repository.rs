use crate::domain::entities::Issue;
use crate::domain::error::DomainResult;
use std::collections::HashMap;

/// Search parameters for issues
#[derive(Debug, Default, Clone)]
pub struct SearchParams {
    pub query: Option<String>,
    pub project_key: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub issue_type: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Repository trait for Issue entity
/// Infrastructure layer will implement this trait
#[allow(dead_code)]
pub trait IssueRepository: Send + Sync {
    fn batch_insert(&self, issues: &[Issue]) -> DomainResult<()>;
    fn find_by_project(&self, project_id: &str) -> DomainResult<Vec<Issue>>;
    fn count_by_project(&self, project_id: &str) -> DomainResult<usize>;
    fn search(&self, params: &SearchParams) -> DomainResult<Vec<Issue>>;
    /// Mark issues as deleted if they are not in the given list of keys (soft delete)
    /// Also unmarks previously deleted issues if they appear in the keys list (restore)
    /// Returns the number of issues marked as deleted
    fn mark_deleted_not_in_keys(&self, project_id: &str, keys: &[String]) -> DomainResult<usize>;

    /// Count issues by status for a project (for integrity check)
    fn count_by_status(&self, project_id: &str) -> DomainResult<HashMap<String, usize>>;
}
