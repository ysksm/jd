//! Metadata handlers

use std::sync::Arc;

use axum::Json;
use jira_db_core::DuckDbMetadataRepository;

use crate::error::ApiError;
use crate::generated::*;
use crate::state::AppState;

/// Get project metadata
pub async fn get(
    state: Arc<AppState>,
    request: MetadataGetRequest,
) -> Result<Json<MetadataGetResponse>, ApiError> {
    let repo = DuckDbMetadataRepository::new(state.db.clone());

    // Get project ID from key
    let project_repo = jira_db_core::DuckDbProjectRepository::new(state.db.clone());
    let project = project_repo
        .find_by_key(&request.project_key)?
        .ok_or_else(|| {
            ApiError::not_found(format!("Project not found: {}", request.project_key))
        })?;

    let statuses = repo.get_statuses(&project.id)?;
    let priorities = repo.get_priorities(&project.id)?;
    let issue_types = repo.get_issue_types(&project.id)?;
    let labels = repo.get_labels(&project.id)?;
    let components = repo.get_components(&project.id)?;
    let fix_versions = repo.get_fix_versions(&project.id)?;

    Ok(Json(MetadataGetResponse {
        metadata: ProjectMetadata {
            project_key: request.project_key,
            statuses: statuses
                .into_iter()
                .map(|s| Status {
                    name: s.name,
                    description: s.description,
                    category: s.category,
                })
                .collect(),
            priorities: priorities
                .into_iter()
                .map(|p| Priority {
                    name: p.name,
                    description: p.description,
                    icon_url: p.icon_url,
                })
                .collect(),
            issue_types: issue_types
                .into_iter()
                .map(|t| IssueType {
                    name: t.name,
                    description: t.description,
                    icon_url: t.icon_url,
                    subtask: t.subtask,
                })
                .collect(),
            labels: labels.into_iter().map(|l| Label { name: l.name }).collect(),
            components: components
                .into_iter()
                .map(|c| Component {
                    name: c.name,
                    description: c.description,
                    lead: c.lead,
                })
                .collect(),
            fix_versions: fix_versions
                .into_iter()
                .map(|v| FixVersion {
                    name: v.name,
                    description: v.description,
                    released: v.released,
                    release_date: v.release_date,
                })
                .collect(),
        },
    }))
}
