use std::sync::Arc;
use crate::domain::entities::Issue;
use crate::domain::error::DomainResult;
use crate::domain::repositories::{IssueRepository, SearchParams};

pub struct SearchIssuesUseCase<I>
where
    I: IssueRepository,
{
    issue_repository: Arc<I>,
}

impl<I> SearchIssuesUseCase<I>
where
    I: IssueRepository,
{
    pub fn new(issue_repository: Arc<I>) -> Self {
        Self { issue_repository }
    }

    pub fn execute(&self, params: SearchParams) -> DomainResult<Vec<Issue>> {
        self.issue_repository.search(&params)
    }
}
