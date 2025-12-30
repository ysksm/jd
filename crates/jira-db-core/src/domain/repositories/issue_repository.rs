use crate::domain::entities::Issue;
use crate::domain::error::DomainResult;

/// Search parameters for issues
#[derive(Debug, Default, Clone)]
pub struct SearchParams {
    pub query: Option<String>,
    pub project_key: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
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
}
