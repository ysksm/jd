use crate::application::dto::TransitionDto;
use crate::application::services::JiraService;
use crate::domain::error::DomainResult;
use std::sync::Arc;

pub struct TransitionIssueUseCase<J>
where
    J: JiraService,
{
    jira_service: Arc<J>,
}

impl<J> TransitionIssueUseCase<J>
where
    J: JiraService,
{
    pub fn new(jira_service: Arc<J>) -> Self {
        Self { jira_service }
    }

    /// Get available transitions for an issue
    pub async fn get_transitions(&self, issue_key: &str) -> DomainResult<Vec<TransitionDto>> {
        self.jira_service.get_issue_transitions(issue_key).await
    }

    /// Transition an issue to a new status
    pub async fn transition(&self, issue_key: &str, transition_id: &str) -> DomainResult<()> {
        self.jira_service
            .transition_issue(issue_key, transition_id)
            .await
    }

    /// Transition multiple issues to a new status
    pub async fn transition_multiple(
        &self,
        issue_keys: &[String],
        transition_id: &str,
    ) -> Vec<TransitionResult> {
        let mut results = Vec::new();

        for issue_key in issue_keys {
            let result = self
                .jira_service
                .transition_issue(issue_key, transition_id)
                .await;

            results.push(TransitionResult {
                issue_key: issue_key.clone(),
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            });
        }

        results
    }
}

/// Result of a transition operation
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub issue_key: String,
    pub success: bool,
    pub error: Option<String>,
}
