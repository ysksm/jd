use crate::domain::entities::{Component, FixVersion, IssueType, Label, Priority, Status};
use crate::domain::error::DomainResult;

/// Repository trait for metadata entities
/// Infrastructure layer will implement this trait
pub trait MetadataRepository: Send + Sync {
    // Status operations
    fn upsert_statuses(&self, project_id: &str, statuses: &[Status]) -> DomainResult<()>;
    fn find_statuses_by_project(&self, project_id: &str) -> DomainResult<Vec<Status>>;

    // Priority operations
    fn upsert_priorities(&self, project_id: &str, priorities: &[Priority]) -> DomainResult<()>;
    fn find_priorities_by_project(&self, project_id: &str) -> DomainResult<Vec<Priority>>;

    // IssueType operations
    fn upsert_issue_types(&self, project_id: &str, issue_types: &[IssueType]) -> DomainResult<()>;
    fn find_issue_types_by_project(&self, project_id: &str) -> DomainResult<Vec<IssueType>>;

    // Label operations
    fn upsert_labels(&self, project_id: &str, labels: &[Label]) -> DomainResult<()>;
    fn find_labels_by_project(&self, project_id: &str) -> DomainResult<Vec<Label>>;

    // Component operations
    fn upsert_components(&self, project_id: &str, components: &[Component]) -> DomainResult<()>;
    fn find_components_by_project(&self, project_id: &str) -> DomainResult<Vec<Component>>;

    // FixVersion operations
    fn upsert_fix_versions(&self, project_id: &str, fix_versions: &[FixVersion]) -> DomainResult<()>;
    fn find_fix_versions_by_project(&self, project_id: &str) -> DomainResult<Vec<FixVersion>>;
}
