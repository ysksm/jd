use std::sync::Arc;
use crate::domain::entities::{Component, FixVersion, IssueType, Label, Priority, Status};
use crate::domain::error::DomainResult;
use crate::domain::repositories::MetadataRepository;

#[derive(Debug, Default)]
pub struct ProjectMetadata {
    pub statuses: Vec<Status>,
    pub priorities: Vec<Priority>,
    pub issue_types: Vec<IssueType>,
    pub labels: Vec<Label>,
    pub components: Vec<Component>,
    pub fix_versions: Vec<FixVersion>,
}

pub struct GetProjectMetadataUseCase<M>
where
    M: MetadataRepository,
{
    metadata_repository: Arc<M>,
}

impl<M> GetProjectMetadataUseCase<M>
where
    M: MetadataRepository,
{
    pub fn new(metadata_repository: Arc<M>) -> Self {
        Self { metadata_repository }
    }

    pub fn execute(&self, project_id: &str) -> DomainResult<ProjectMetadata> {
        Ok(ProjectMetadata {
            statuses: self.metadata_repository.find_statuses_by_project(project_id)?,
            priorities: self.metadata_repository.find_priorities_by_project(project_id)?,
            issue_types: self.metadata_repository.find_issue_types_by_project(project_id)?,
            labels: self.metadata_repository.find_labels_by_project(project_id)?,
            components: self.metadata_repository.find_components_by_project(project_id)?,
            fix_versions: self.metadata_repository.find_fix_versions_by_project(project_id)?,
        })
    }

    pub fn execute_by_type(&self, project_id: &str, metadata_type: &str) -> DomainResult<ProjectMetadata> {
        let mut metadata = ProjectMetadata::default();

        match metadata_type {
            "status" => metadata.statuses = self.metadata_repository.find_statuses_by_project(project_id)?,
            "priority" => metadata.priorities = self.metadata_repository.find_priorities_by_project(project_id)?,
            "issue-type" => metadata.issue_types = self.metadata_repository.find_issue_types_by_project(project_id)?,
            "label" => metadata.labels = self.metadata_repository.find_labels_by_project(project_id)?,
            "component" => metadata.components = self.metadata_repository.find_components_by_project(project_id)?,
            "version" => metadata.fix_versions = self.metadata_repository.find_fix_versions_by_project(project_id)?,
            _ => return self.execute(project_id),
        }

        Ok(metadata)
    }
}
