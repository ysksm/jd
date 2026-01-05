//! Metadata command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{DuckDbMetadataRepository, GetProjectMetadataUseCase};

use crate::generated::*;
use crate::state::AppState;

/// Get project metadata
#[tauri::command]
pub async fn metadata_get(
    state: State<'_, AppState>,
    request: MetadataGetRequest,
) -> Result<MetadataGetResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db(&request.project_key).ok_or_else(|| {
        format!(
            "Database not initialized for project {}",
            request.project_key
        )
    })?;

    // Find project ID from key
    let project = settings
        .projects
        .iter()
        .find(|p| p.key == request.project_key)
        .ok_or("Project not found")?;

    // Create repository and use case
    let metadata_repo = Arc::new(DuckDbMetadataRepository::new(db));
    let use_case = GetProjectMetadataUseCase::new(metadata_repo);

    // Execute
    let metadata = if let Some(ref type_filter) = request.r#type {
        use_case
            .execute_by_type(&project.id, type_filter)
            .map_err(|e| e.to_string())?
    } else {
        use_case.execute(&project.id).map_err(|e| e.to_string())?
    };

    // Convert to generated types
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
