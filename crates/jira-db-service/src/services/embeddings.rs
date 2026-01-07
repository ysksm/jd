//! Embeddings service

use std::sync::Arc;

use jira_db_core::{
    DuckDbIssueRepository, EmbeddingGenerationConfig, EmbeddingProviderType, EmbeddingsRepository,
    GenerateEmbeddingsUseCase, ProviderConfig, SearchIssuesUseCase, SearchParams, create_provider,
};

use crate::error::{ServiceError, ServiceResult};
use crate::state::AppState;
use crate::types::*;

/// Parse provider string to EmbeddingProviderType
fn parse_provider(provider_str: &str) -> ServiceResult<EmbeddingProviderType> {
    provider_str
        .parse::<EmbeddingProviderType>()
        .map_err(|e| ServiceError::InvalidRequest(e.to_string()))
}

/// Generate embeddings for semantic search
pub async fn generate(
    state: &AppState,
    request: EmbeddingsGenerateRequest,
) -> ServiceResult<EmbeddingsGenerateResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Get embedding config from settings
    let embedding_config = settings.embeddings.as_ref().ok_or_else(|| {
        ServiceError::Config("Embedding configuration not found in settings".to_string())
    })?;

    // Determine provider type from request or settings
    let provider_str = request
        .provider
        .as_deref()
        .unwrap_or(&embedding_config.provider);
    let provider_type = parse_provider(provider_str)?;

    // Create provider config
    let provider_config = ProviderConfig {
        provider: provider_type,
        model: Some(embedding_config.model.clone()),
        endpoint: embedding_config.endpoint.clone(),
        api_key: None, // Will use env var
    };

    // Create embedding provider
    let embedding_provider = Arc::new(
        create_provider(provider_config).map_err(|e| ServiceError::Internal(e.to_string()))?,
    );

    // Create repositories
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let embeddings_repo = Arc::new(EmbeddingsRepository::new(db));

    // Configure
    let config = EmbeddingGenerationConfig {
        batch_size: request.batch_size.unwrap_or(50) as usize,
        force_regenerate: request.force.unwrap_or(false),
    };

    // Create use case
    let use_case =
        GenerateEmbeddingsUseCase::new(issue_repo, embeddings_repo, embedding_provider, config);

    // Execute with optional project key filter
    let result = use_case
        .execute(request.project_key.as_deref())
        .await
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

    Ok(EmbeddingsGenerateResponse {
        stats: EmbeddingStats {
            total_issues: result.total_issues as i32,
            processed_issues: result.embeddings_generated as i32,
            duration: result.duration_secs,
        },
    })
}

/// Semantic search using embeddings
pub async fn search(
    state: &AppState,
    request: SemanticSearchRequest,
) -> ServiceResult<SemanticSearchResponse> {
    let settings = state.get_settings().ok_or(ServiceError::NotInitialized)?;
    let db = state.get_db().ok_or(ServiceError::NotInitialized)?;

    // Get embedding config from settings
    let embedding_config = settings.embeddings.as_ref().ok_or_else(|| {
        ServiceError::Config("Embedding configuration not found in settings".to_string())
    })?;

    // Parse provider type
    let provider_type = parse_provider(&embedding_config.provider)?;

    // Create provider config
    let provider_config = ProviderConfig {
        provider: provider_type,
        model: Some(embedding_config.model.clone()),
        endpoint: embedding_config.endpoint.clone(),
        api_key: None,
    };

    // Create embedding provider
    let embedding_provider =
        create_provider(provider_config).map_err(|e| ServiceError::Internal(e.to_string()))?;

    // Generate query embedding
    let query_embedding = embedding_provider
        .embed(&request.query)
        .await
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

    // Create embeddings repository
    let embeddings_repo = EmbeddingsRepository::new(db.clone());

    // Get issue repository for fetching full issues
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db));
    let search_use_case = SearchIssuesUseCase::new(issue_repo);

    // Perform semantic search
    let search_results = embeddings_repo
        .semantic_search(
            &query_embedding,
            request.project_key.as_deref(),
            request.limit.unwrap_or(10) as usize,
        )
        .map_err(|e| ServiceError::Database(e.to_string()))?;

    // Convert to response format with full issue data
    let mut results = Vec::new();
    for sr in search_results {
        // Search for the issue by key
        let params = SearchParams {
            query: Some(sr.issue_key.clone()),
            project_key: None,
            status: None,
            assignee: None,
            issue_type: None,
            priority: None,
            team: None,
            limit: Some(1),
            offset: None,
        };

        if let Ok(issues) = search_use_case.execute(params) {
            if let Some(issue) = issues.into_iter().find(|i| i.key == sr.issue_key) {
                results.push(SemanticSearchResult {
                    issue: Issue {
                        id: issue.id,
                        key: issue.key.clone(),
                        project_key: issue.key.split('-').next().unwrap_or("").to_string(),
                        summary: issue.summary,
                        description: issue.description,
                        status: issue.status.unwrap_or_default(),
                        priority: issue.priority.unwrap_or_default(),
                        issue_type: issue.issue_type.unwrap_or_default(),
                        assignee: issue.assignee,
                        reporter: issue.reporter,
                        parent_key: issue.parent_key,
                        labels: issue.labels.unwrap_or_default(),
                        components: issue.components.unwrap_or_default(),
                        fix_versions: issue.fix_versions.unwrap_or_default(),
                        created_at: issue.created_date.unwrap_or_else(chrono::Utc::now),
                        updated_at: issue.updated_date.unwrap_or_else(chrono::Utc::now),
                    },
                    score: sr.similarity_score as f64,
                });
            }
        }
    }

    Ok(SemanticSearchResponse { results })
}
