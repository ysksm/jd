//! Metadata service

use std::sync::Arc;

use jira_db_core::{DuckDbMetadataRepository, GetProjectMetadataUseCase};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// Get project metadata
pub fn get(state: &AppState, request: MetadataGetRequest) -> ServiceResult<MetadataGetResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Find project ID from key
    let project = settings
        .projects
        .iter()
        .find(|p| p.key == request.project_key)
        .ok_or_else(|| ServiceError::NotFound("Project not found".to_string()))?;

    // Create repository and use case
    let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db));
    let use_case = GetProjectMetadataUseCase::new(metadata_repo);

    // Execute
    let metadata = if let Some(ref type_filter) = request.metadata_type {
        use_case.execute_by_type(&project.id, type_filter)?
    } else {
        use_case.execute(&project.id)?
    };

    // Convert to API types
    Ok(MetadataGetResponse {
        metadata: ProjectMetadata {
            project_key: request.project_key,
            statuses: metadata
                .statuses
                .into_iter()
                .map(|s| Status {
                    name: s.name,
                    description: s.description,
                    category: s.category.unwrap_or_default(),
                })
                .collect(),
            priorities: metadata
                .priorities
                .into_iter()
                .map(|p| Priority {
                    name: p.name,
                    description: p.description,
                    icon_url: p.icon_url,
                })
                .collect(),
            issue_types: metadata
                .issue_types
                .into_iter()
                .map(|it| IssueType {
                    name: it.name,
                    description: it.description,
                    icon_url: it.icon_url,
                    subtask: it.subtask,
                })
                .collect(),
            labels: metadata
                .labels
                .into_iter()
                .map(|l| Label { name: l.name })
                .collect(),
            components: metadata
                .components
                .into_iter()
                .map(|c| Component {
                    name: c.name,
                    description: c.description,
                    lead: c.lead,
                })
                .collect(),
            fix_versions: metadata
                .fix_versions
                .into_iter()
                .map(|fv| FixVersion {
                    name: fv.name,
                    description: fv.description,
                    released: fv.released,
                    release_date: fv.release_date,
                })
                .collect(),
        },
    })
}
