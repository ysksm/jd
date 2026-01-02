use crate::application::services::JiraService;
use crate::domain::entities::Project;
use crate::domain::error::DomainResult;
use crate::domain::repositories::ProjectRepository;
use log::info;
use std::sync::Arc;

pub struct SyncProjectListUseCase<P, J>
where
    P: ProjectRepository,
    J: JiraService,
{
    project_repository: Arc<P>,
    jira_service: Arc<J>,
}

impl<P, J> SyncProjectListUseCase<P, J>
where
    P: ProjectRepository,
    J: JiraService,
{
    pub fn new(project_repository: Arc<P>, jira_service: Arc<J>) -> Self {
        Self {
            project_repository,
            jira_service,
        }
    }

    pub async fn execute(&self) -> DomainResult<Vec<Project>> {
        info!("Fetching project list from JIRA...");

        let projects = self.jira_service.fetch_projects().await?;
        info!("Found {} projects", projects.len());

        for project in &projects {
            self.project_repository.insert(project)?;
        }

        Ok(projects)
    }
}
