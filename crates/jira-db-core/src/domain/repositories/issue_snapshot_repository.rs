use crate::domain::entities::IssueSnapshot;
use crate::domain::error::DomainResult;

/// Repository trait for IssueSnapshot entity
/// Infrastructure layer will implement this trait
pub trait IssueSnapshotRepository: Send + Sync {
    /// Insert multiple snapshots in batch
    fn batch_insert(&self, snapshots: &[IssueSnapshot]) -> DomainResult<()>;

    /// Delete all snapshots for a specific issue
    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()>;

    /// Delete all snapshots for a specific project
    fn delete_by_project_id(&self, project_id: &str) -> DomainResult<()>;

    /// Find all snapshots for an issue, ordered by version
    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<IssueSnapshot>>;

    /// Find snapshot at a specific version
    fn find_by_issue_key_and_version(
        &self,
        issue_key: &str,
        version: i32,
    ) -> DomainResult<Option<IssueSnapshot>>;

    /// Find the current (latest) snapshot for an issue
    fn find_current_by_issue_key(&self, issue_key: &str) -> DomainResult<Option<IssueSnapshot>>;

    /// Find all snapshots for a project
    fn find_by_project_id(&self, project_id: &str) -> DomainResult<Vec<IssueSnapshot>>;

    /// Count snapshots for an issue
    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize>;

    /// Count total snapshots for a project
    fn count_by_project_id(&self, project_id: &str) -> DomainResult<usize>;
}
