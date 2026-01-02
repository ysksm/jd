use crate::domain::entities::ChangeHistoryItem;
use crate::domain::error::DomainResult;
use crate::domain::repositories::ChangeHistoryRepository;
use std::sync::Arc;

pub struct GetChangeHistoryUseCase<C>
where
    C: ChangeHistoryRepository,
{
    change_history_repository: Arc<C>,
}

impl<C> GetChangeHistoryUseCase<C>
where
    C: ChangeHistoryRepository,
{
    pub fn new(change_history_repository: Arc<C>) -> Self {
        Self {
            change_history_repository,
        }
    }

    pub fn execute(
        &self,
        issue_key: &str,
        field_filter: Option<&str>,
    ) -> DomainResult<Vec<ChangeHistoryItem>> {
        self.change_history_repository
            .find_by_issue_key_and_field(issue_key, field_filter)
    }

    pub fn count(&self, issue_key: &str) -> DomainResult<usize> {
        self.change_history_repository.count_by_issue_key(issue_key)
    }
}
