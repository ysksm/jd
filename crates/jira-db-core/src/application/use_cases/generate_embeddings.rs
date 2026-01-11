//! Generate Embeddings Use Case
//!
//! Generates vector embeddings for issues using OpenAI API.
//! Embeddings are created from concatenated text fields for semantic search.

use std::sync::Arc;
use std::time::Instant;

use log::{info, warn};

use crate::domain::entities::Issue;
use crate::domain::error::DomainResult;
use crate::domain::repositories::IssueRepository;
use crate::infrastructure::database::EmbeddingsRepository;
use crate::infrastructure::external::embeddings::EmbeddingProvider;

/// Configuration for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingGenerationConfig {
    /// Batch size for API calls
    pub batch_size: usize,
    /// Whether to regenerate existing embeddings
    pub force_regenerate: bool,
}

impl Default for EmbeddingGenerationConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            force_regenerate: false,
        }
    }
}

/// Result of embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingGenerationResult {
    /// Total issues processed
    pub total_issues: usize,
    /// New embeddings generated
    pub embeddings_generated: usize,
    /// Embeddings skipped (already exist)
    pub embeddings_skipped: usize,
    /// Errors encountered
    pub errors: usize,
    /// Total time taken
    pub duration_secs: f64,
    /// Time breakdown
    pub timing: EmbeddingTiming,
}

/// Timing breakdown for embedding generation
#[derive(Debug, Clone, Default)]
pub struct EmbeddingTiming {
    /// Time spent fetching issues
    pub fetch_issues_secs: f64,
    /// Time spent calling embedding API
    pub embedding_api_secs: f64,
    /// Time spent storing embeddings
    pub store_embeddings_secs: f64,
}

/// Use case for generating embeddings for issues
pub struct GenerateEmbeddingsUseCase<I, E>
where
    I: IssueRepository,
    E: EmbeddingProvider,
{
    issue_repository: Arc<I>,
    embeddings_repository: Arc<EmbeddingsRepository>,
    embedding_provider: Arc<E>,
    config: EmbeddingGenerationConfig,
}

impl<I, E> GenerateEmbeddingsUseCase<I, E>
where
    I: IssueRepository,
    E: EmbeddingProvider,
{
    /// Create a new use case instance
    pub fn new(
        issue_repository: Arc<I>,
        embeddings_repository: Arc<EmbeddingsRepository>,
        embedding_provider: Arc<E>,
        config: EmbeddingGenerationConfig,
    ) -> Self {
        Self {
            issue_repository,
            embeddings_repository,
            embedding_provider,
            config,
        }
    }

    /// Generate embeddings for all issues in a project
    pub async fn execute(
        &self,
        project_key: Option<&str>,
    ) -> DomainResult<EmbeddingGenerationResult> {
        let total_start = Instant::now();
        let mut timing = EmbeddingTiming::default();

        // Initialize embeddings schema if needed
        self.embeddings_repository.init_schema()?;

        // Fetch issues
        let fetch_start = Instant::now();
        let search_params = crate::domain::repositories::SearchParams {
            project_key: project_key.map(|s| s.to_string()),
            limit: Some(10000), // Reasonable max limit
            ..Default::default()
        };
        let issues = match project_key {
            Some(key) => {
                info!("Fetching issues for project: {}", key);
                self.issue_repository.search(&search_params)?
            }
            None => {
                info!("Fetching all issues...");
                self.issue_repository.search(&search_params)?
            }
        };
        timing.fetch_issues_secs = fetch_start.elapsed().as_secs_f64();

        let total_issues = issues.len();
        info!(
            "Found {} issues to process (fetch took {:.2}s)",
            total_issues, timing.fetch_issues_secs
        );

        if total_issues == 0 {
            return Ok(EmbeddingGenerationResult {
                total_issues: 0,
                embeddings_generated: 0,
                embeddings_skipped: 0,
                errors: 0,
                duration_secs: total_start.elapsed().as_secs_f64(),
                timing,
            });
        }

        // Filter issues that need embeddings
        let issues_to_process: Vec<&Issue> = if self.config.force_regenerate {
            issues.iter().collect()
        } else {
            issues
                .iter()
                .filter(|i| !self.embeddings_repository.exists(&i.id).unwrap_or(true))
                .collect()
        };

        let skipped = total_issues - issues_to_process.len();
        info!(
            "{} issues need embedding generation, {} already have embeddings",
            issues_to_process.len(),
            skipped
        );

        if issues_to_process.is_empty() {
            return Ok(EmbeddingGenerationResult {
                total_issues,
                embeddings_generated: 0,
                embeddings_skipped: skipped,
                errors: 0,
                duration_secs: total_start.elapsed().as_secs_f64(),
                timing,
            });
        }

        // Process in batches
        let mut embeddings_generated = 0;
        let mut errors = 0;

        for (batch_idx, batch) in issues_to_process.chunks(self.config.batch_size).enumerate() {
            info!(
                "Processing batch {}/{} ({} issues)",
                batch_idx + 1,
                (issues_to_process.len() + self.config.batch_size - 1) / self.config.batch_size,
                batch.len()
            );

            // Prepare texts for embedding
            let texts: Vec<String> = batch
                .iter()
                .map(|issue| Self::create_embedding_text(issue))
                .collect();
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

            // Generate embeddings
            let api_start = Instant::now();
            match self.embedding_provider.embed_batch(&text_refs).await {
                Ok(embeddings) => {
                    timing.embedding_api_secs += api_start.elapsed().as_secs_f64();

                    // Store embeddings
                    let store_start = Instant::now();
                    let provider_name = self.embedding_provider.provider_name();
                    let model_name = self.embedding_provider.model_name();
                    for (issue, embedding) in batch.iter().zip(embeddings.iter()) {
                        match self.embeddings_repository.upsert_embedding(
                            &issue.id,
                            &issue.key,
                            embedding,
                            &texts[batch.iter().position(|i| i.id == issue.id).unwrap()],
                            provider_name,
                            model_name,
                        ) {
                            Ok(_) => embeddings_generated += 1,
                            Err(e) => {
                                warn!("Failed to store embedding for {}: {}", issue.key, e);
                                errors += 1;
                            }
                        }
                    }
                    timing.store_embeddings_secs += store_start.elapsed().as_secs_f64();
                }
                Err(e) => {
                    warn!("Failed to generate embeddings for batch: {}", e);
                    errors += batch.len();
                }
            }
        }

        let duration_secs = total_start.elapsed().as_secs_f64();

        info!(
            "Embedding generation complete: {} generated, {} skipped, {} errors in {:.2}s",
            embeddings_generated, skipped, errors, duration_secs
        );
        info!(
            "Timing breakdown: fetch {:.2}s, API {:.2}s, store {:.2}s",
            timing.fetch_issues_secs, timing.embedding_api_secs, timing.store_embeddings_secs
        );

        Ok(EmbeddingGenerationResult {
            total_issues,
            embeddings_generated,
            embeddings_skipped: skipped,
            errors,
            duration_secs,
            timing,
        })
    }

    /// Create text for embedding from issue fields
    fn create_embedding_text(issue: &Issue) -> String {
        let mut parts = Vec::new();

        // Key
        parts.push(format!("Key: {}", issue.key));

        // Summary (most important)
        parts.push(format!("Summary: {}", issue.summary));

        // Description
        if let Some(desc) = &issue.description {
            if !desc.is_empty() {
                parts.push(format!("Description: {}", desc));
            }
        }

        // Status
        if let Some(status) = &issue.status {
            parts.push(format!("Status: {}", status));
        }

        // Priority
        if let Some(priority) = &issue.priority {
            parts.push(format!("Priority: {}", priority));
        }

        // Issue type
        if let Some(issue_type) = &issue.issue_type {
            parts.push(format!("Type: {}", issue_type));
        }

        // Assignee
        if let Some(assignee) = &issue.assignee {
            parts.push(format!("Assignee: {}", assignee));
        }

        // Reporter
        if let Some(reporter) = &issue.reporter {
            parts.push(format!("Reporter: {}", reporter));
        }

        // Labels
        if let Some(labels) = &issue.labels {
            if !labels.is_empty() {
                parts.push(format!("Labels: {}", labels.join(", ")));
            }
        }

        // Components
        if let Some(components) = &issue.components {
            if !components.is_empty() {
                parts.push(format!("Components: {}", components.join(", ")));
            }
        }

        // Sprint
        if let Some(sprint) = &issue.sprint {
            if !sprint.is_empty() {
                parts.push(format!("Sprint: {}", sprint));
            }
        }

        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Issue;
    use chrono::Utc;

    #[test]
    fn test_create_embedding_text() {
        let issue = Issue {
            id: "123".to_string(),
            project_id: "PROJ".to_string(),
            key: "PROJ-123".to_string(),
            summary: "Fix login bug".to_string(),
            description: Some("Users cannot login with SSO".to_string()),
            status: Some("Open".to_string()),
            priority: Some("High".to_string()),
            issue_type: Some("Bug".to_string()),
            assignee: Some("john.doe".to_string()),
            reporter: Some("jane.doe".to_string()),
            resolution: None,
            labels: Some(vec!["login".to_string(), "sso".to_string()]),
            components: Some(vec!["auth".to_string()]),
            fix_versions: None,
            sprint: None,
            team: None,
            parent_key: None,
            due_date: None,
            created_date: Some(Utc::now()),
            updated_date: Some(Utc::now()),
            raw_json: None,
        };

        let text = GenerateEmbeddingsUseCase::<
            crate::DuckDbIssueRepository,
            crate::OpenAIEmbeddingClient,
        >::create_embedding_text(&issue);

        assert!(text.contains("PROJ-123"));
        assert!(text.contains("Fix login bug"));
        assert!(text.contains("Users cannot login with SSO"));
        assert!(text.contains("Status: Open"));
        assert!(text.contains("Priority: High"));
        assert!(text.contains("Type: Bug"));
        assert!(text.contains("Labels: login, sso"));
    }
}
