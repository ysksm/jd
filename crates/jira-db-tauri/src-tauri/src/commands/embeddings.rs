//! Embeddings command handlers

use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    create_provider, DuckDbIssueRepository, EmbeddingGenerationConfig, EmbeddingsRepository,
    GenerateEmbeddingsUseCase, IssueRepository, ProviderConfig,
};

use crate::generated::*;
use crate::state::AppState;

/// Generate embeddings for semantic search
#[tauri::command]
pub async fn embeddings_generate(
    state: State<'_, AppState>,
    request: EmbeddingsGenerateRequest,
) -> Result<EmbeddingsGenerateResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Get embedding config from settings
    let embedding_config = settings
        .embeddings
        .as_ref()
        .ok_or("Embedding configuration not found in settings")?;

    // Create provider config
    let provider_config = ProviderConfig {
        provider: request
            .provider
            .unwrap_or_else(|| embedding_config.provider.clone()),
        model: request
            .model_name
            .or_else(|| embedding_config.model.clone()),
        endpoint: embedding_config.endpoint.clone(),
        api_key: None, // Will use env var
    };

    // Create embedding provider
    let embedding_provider = Arc::new(
        create_provider(provider_config)
            .await
            .map_err(|e| e.to_string())?,
    );

    // Create repositories
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let embeddings_repo = Arc::new(EmbeddingsRepository::new(db).map_err(|e| e.to_string())?);

    // Configure
    let config = EmbeddingGenerationConfig {
        batch_size: request.batch_size.unwrap_or(50) as usize,
        force_regenerate: request.force.unwrap_or(false),
    };

    // Create use case
    let use_case = GenerateEmbeddingsUseCase::new(
        issue_repo,
        embeddings_repo,
        embedding_provider,
        config,
    );

    // Get projects to process
    let projects_to_process: Vec<_> = if let Some(ref project_key) = request.project_key {
        settings
            .projects
            .iter()
            .filter(|p| &p.key == project_key)
            .collect()
    } else {
        settings
            .projects
            .iter()
            .filter(|p| p.sync_enabled)
            .collect()
    };

    if projects_to_process.is_empty() {
        return Err("No projects to process".to_string());
    }

    let project_ids: Vec<_> = projects_to_process.iter().map(|p| p.id.as_str()).collect();

    // Execute
    let result = use_case
        .execute(&project_ids)
        .await
        .map_err(|e| e.to_string())?;

    Ok(EmbeddingsGenerateResponse {
        stats: EmbeddingStats {
            total_issues: result.total_issues as i32,
            processed_issues: result.embeddings_generated as i32,
            duration: result.duration_secs,
        },
    })
}

/// Semantic search using embeddings
#[tauri::command]
pub async fn embeddings_search(
    state: State<'_, AppState>,
    request: SemanticSearchRequest,
) -> Result<SemanticSearchResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Get embedding config from settings
    let embedding_config = settings
        .embeddings
        .as_ref()
        .ok_or("Embedding configuration not found in settings")?;

    // Create provider config
    let provider_config = ProviderConfig {
        provider: embedding_config.provider.clone(),
        model: embedding_config.model.clone(),
        endpoint: embedding_config.endpoint.clone(),
        api_key: None,
    };

    // Create embedding provider
    let embedding_provider = create_provider(provider_config)
        .await
        .map_err(|e| e.to_string())?;

    // Generate query embedding
    let query_embedding = embedding_provider
        .embed(&request.query)
        .await
        .map_err(|e| e.to_string())?;

    // Create embeddings repository
    let embeddings_repo = EmbeddingsRepository::new(db.clone()).map_err(|e| e.to_string())?;

    // Get issue repository for fetching full issues
    let issue_repo = DuckDbIssueRepository::new(db);

    // Perform semantic search
    let search_results = embeddings_repo
        .semantic_search(
            &query_embedding,
            request.project_key.as_deref(),
            request.limit.unwrap_or(10) as usize,
        )
        .map_err(|e| e.to_string())?;

    // Convert to response format with full issue data
    let mut results = Vec::new();
    for sr in search_results {
        if let Ok(Some(issue)) = issue_repo.find_by_key(&sr.issue_key) {
            results.push(SemanticSearchResult {
                issue: Issue {
                    id: issue.id,
                    key: issue.key,
                    project_key: issue.project_key,
                    summary: issue.summary,
                    description: issue.description,
                    status: issue.status,
                    priority: issue.priority,
                    issue_type: issue.issue_type,
                    assignee: issue.assignee,
                    reporter: issue.reporter,
                    labels: issue.labels.unwrap_or_default(),
                    components: issue.components.unwrap_or_default(),
                    fix_versions: issue.fix_versions.unwrap_or_default(),
                    created_at: issue.created_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
                    updated_at: issue.updated_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
                },
                score: sr.similarity,
            });
        }
    }

    Ok(SemanticSearchResponse { results })
}
