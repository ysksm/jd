use crate::application::dto::CreatedIssueDto;
use crate::application::services::JiraService;
use crate::domain::error::DomainResult;
use std::sync::Arc;

pub struct CreateTestTicketUseCase<J>
where
    J: JiraService,
{
    jira_service: Arc<J>,
}

impl<J> CreateTestTicketUseCase<J>
where
    J: JiraService,
{
    pub fn new(jira_service: Arc<J>) -> Self {
        Self { jira_service }
    }

    pub async fn execute(
        &self,
        project_key: &str,
        summary: &str,
        description: Option<&str>,
        issue_type: &str,
    ) -> DomainResult<CreatedIssueDto> {
        self.jira_service
            .create_issue(project_key, summary, description, issue_type)
            .await
    }
}
