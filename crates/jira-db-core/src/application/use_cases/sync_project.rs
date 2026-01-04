use crate::application::dto::SyncResult;
use crate::application::services::JiraService;
use crate::application::use_cases::GenerateSnapshotsUseCase;
use crate::domain::entities::ChangeHistoryItem;
use crate::domain::error::DomainResult;
use crate::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository, MetadataRepository,
    SyncHistoryRepository,
};
use chrono::Utc;
use log::{info, warn};
use std::sync::Arc;

pub struct SyncProjectUseCase<I, C, M, S, N, J>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
    M: MetadataRepository,
    S: SyncHistoryRepository,
    N: IssueSnapshotRepository,
    J: JiraService,
{
    issue_repository: Arc<I>,
    change_history_repository: Arc<C>,
    metadata_repository: Arc<M>,
    sync_history_repository: Arc<S>,
    snapshot_repository: Arc<N>,
    jira_service: Arc<J>,
}

impl<I, C, M, S, N, J> SyncProjectUseCase<I, C, M, S, N, J>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
    M: MetadataRepository,
    S: SyncHistoryRepository,
    N: IssueSnapshotRepository,
    J: JiraService,
{
    pub fn new(
        issue_repository: Arc<I>,
        change_history_repository: Arc<C>,
        metadata_repository: Arc<M>,
        sync_history_repository: Arc<S>,
        snapshot_repository: Arc<N>,
        jira_service: Arc<J>,
    ) -> Self {
        Self {
            issue_repository,
            change_history_repository,
            metadata_repository,
            sync_history_repository,
            snapshot_repository,
            jira_service,
        }
    }

    pub async fn execute(&self, project_key: &str, project_id: &str) -> DomainResult<SyncResult> {
        info!("Syncing project: {}", project_key);

        let started_at = Utc::now();
        let history_id = self
            .sync_history_repository
            .insert(project_id, "full", started_at)?;

        match self.sync_internal(project_key, project_id).await {
            Ok((issues_count, history_count)) => {
                let completed_at = Utc::now();
                self.sync_history_repository.update_completed(
                    history_id,
                    issues_count,
                    completed_at,
                )?;

                info!(
                    "Successfully synced {} issues for project {}",
                    issues_count, project_key
                );
                Ok(SyncResult::success(
                    project_key.to_string(),
                    issues_count,
                    history_count,
                ))
            }
            Err(e) => {
                let completed_at = Utc::now();
                self.sync_history_repository.update_failed(
                    history_id,
                    &e.to_string(),
                    completed_at,
                )?;
                Ok(SyncResult::failure(project_key.to_string(), e.to_string()))
            }
        }
    }

    async fn sync_internal(
        &self,
        project_key: &str,
        project_id: &str,
    ) -> DomainResult<(usize, usize)> {
        info!("Fetching issues for project: {}", project_key);

        let issues = self.jira_service.fetch_project_issues(project_key).await?;
        let count = issues.len();

        info!("Fetched {} issues, saving to database...", count);

        // Save issues in chunks
        let chunk_size = 50;
        for chunk in issues.chunks(chunk_size) {
            self.issue_repository.batch_insert(chunk)?;
        }

        // Mark issues that no longer exist in JIRA as deleted (soft delete)
        let issue_keys: Vec<String> = issues.iter().map(|i| i.key.clone()).collect();
        let deleted_count = self
            .issue_repository
            .mark_deleted_not_in_keys(project_id, &issue_keys)?;
        if deleted_count > 0 {
            info!(
                "Marked {} issues as deleted (no longer in JIRA)",
                deleted_count
            );
        }

        // Extract and save change history
        info!("Extracting and saving change history...");
        let mut total_history_items = 0;

        for issue in &issues {
            if let Some(raw_json) = &issue.raw_json {
                self.change_history_repository
                    .delete_by_issue_id(&issue.id)?;

                let history_items =
                    ChangeHistoryItem::extract_from_raw_json(&issue.id, &issue.key, raw_json);

                if !history_items.is_empty() {
                    info!(
                        "  {} has {} change history items",
                        issue.key,
                        history_items.len()
                    );
                    self.change_history_repository
                        .batch_insert(&history_items)?;
                    total_history_items += history_items.len();
                }
            } else {
                warn!("  {} has no raw_json", issue.key);
            }
        }

        if total_history_items > 0 {
            info!("Saved {} change history items", total_history_items);
        }

        // Fetch and save metadata
        self.sync_metadata(project_key, project_id).await?;

        // Generate issue snapshots
        info!("Generating issue snapshots...");
        let snapshot_use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&self.issue_repository),
            Arc::clone(&self.change_history_repository),
            Arc::clone(&self.snapshot_repository),
        );
        match snapshot_use_case.execute(project_key, project_id) {
            Ok(result) => {
                info!(
                    "Generated {} snapshots for {} issues",
                    result.snapshots_generated, result.issues_processed
                );
            }
            Err(e) => {
                warn!("Failed to generate snapshots: {}", e);
            }
        }

        Ok((count, total_history_items))
    }

    async fn sync_metadata(&self, project_key: &str, project_id: &str) -> DomainResult<()> {
        info!("Fetching and saving project metadata...");

        // Fetch statuses
        match self.jira_service.fetch_project_statuses(project_key).await {
            Ok(statuses) => {
                if !statuses.is_empty() {
                    self.metadata_repository
                        .upsert_statuses(project_id, &statuses)?;
                    info!("Saved {} statuses", statuses.len());
                }
            }
            Err(e) => warn!("Failed to fetch statuses: {}", e),
        }

        // Fetch priorities
        match self.jira_service.fetch_priorities().await {
            Ok(priorities) => {
                if !priorities.is_empty() {
                    self.metadata_repository
                        .upsert_priorities(project_id, &priorities)?;
                    info!("Saved {} priorities", priorities.len());
                }
            }
            Err(e) => warn!("Failed to fetch priorities: {}", e),
        }

        // Fetch issue types
        match self
            .jira_service
            .fetch_project_issue_types(project_id)
            .await
        {
            Ok(issue_types) => {
                if !issue_types.is_empty() {
                    self.metadata_repository
                        .upsert_issue_types(project_id, &issue_types)?;
                    info!("Saved {} issue types", issue_types.len());
                }
            }
            Err(e) => warn!("Failed to fetch issue types: {}", e),
        }

        // Fetch labels
        match self.jira_service.fetch_project_labels(project_key).await {
            Ok(labels) => {
                if !labels.is_empty() {
                    self.metadata_repository
                        .upsert_labels(project_id, &labels)?;
                    info!("Saved {} labels", labels.len());
                }
            }
            Err(e) => warn!("Failed to fetch labels: {}", e),
        }

        // Fetch components
        match self
            .jira_service
            .fetch_project_components(project_key)
            .await
        {
            Ok(components) => {
                if !components.is_empty() {
                    self.metadata_repository
                        .upsert_components(project_id, &components)?;
                    info!("Saved {} components", components.len());
                }
            }
            Err(e) => warn!("Failed to fetch components: {}", e),
        }

        // Fetch versions
        match self.jira_service.fetch_project_versions(project_key).await {
            Ok(fix_versions) => {
                if !fix_versions.is_empty() {
                    self.metadata_repository
                        .upsert_fix_versions(project_id, &fix_versions)?;
                    info!("Saved {} fix versions", fix_versions.len());
                }
            }
            Err(e) => warn!("Failed to fetch versions: {}", e),
        }

        Ok(())
    }
}
