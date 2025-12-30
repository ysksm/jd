use crate::domain::entities::ChangeHistoryItem;
use crate::domain::error::DomainResult;

/// Repository trait for ChangeHistoryItem entity
/// Infrastructure layer will implement this trait
#[allow(dead_code)]
pub trait ChangeHistoryRepository: Send + Sync {
    fn batch_insert(&self, items: &[ChangeHistoryItem]) -> DomainResult<()>;
    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()>;
    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<ChangeHistoryItem>>;
    fn find_by_issue_key_and_field(
        &self,
        issue_key: &str,
        field_filter: Option<&str>,
    ) -> DomainResult<Vec<ChangeHistoryItem>>;
    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize>;
}
