use crate::application::dto::SyncResult;
use crate::application::services::JiraService;
use crate::application::use_cases::GenerateSnapshotsUseCase;
use crate::domain::entities::{ChangeHistoryItem, Issue};
use crate::domain::error::DomainResult;
use crate::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository, MetadataRepository,
    SyncHistoryRepository,
};
use crate::infrastructure::config::SyncCheckpoint;
use crate::infrastructure::database::SharedRawDataRepository;
use chrono::Utc;
use log::{info, warn};
use std::sync::Arc;

/// Result of resumable sync operation
#[derive(Debug)]
pub struct ResumableSyncResult {
    pub sync_result: SyncResult,
    /// Checkpoint to save (Some if sync failed and can be resumed, None if completed)
    pub checkpoint: Option<SyncCheckpoint>,
}

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
    /// Optional repository for storing raw JIRA API JSON data in a separate database
    raw_repository: Option<SharedRawDataRepository>,
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
            raw_repository: None,
        }
    }

    /// Set the raw data repository for storing JIRA API JSON in a separate database
    pub fn with_raw_repository(mut self, raw_repository: SharedRawDataRepository) -> Self {
        self.raw_repository = Some(raw_repository);
        self
    }

    pub async fn execute(&self, project_key: &str, project_id: &str) -> DomainResult<SyncResult> {
        // Use resumable sync without checkpoint (full sync)
        let result = self
            .execute_resumable(project_key, project_id, None, |_| {})
            .await?;
        Ok(result.sync_result)
    }

    /// Execute resumable sync with checkpoint support
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `project_id` - The JIRA project ID
    /// * `checkpoint` - Optional checkpoint to resume from
    /// * `on_progress` - Callback called after each batch with the new checkpoint
    ///
    /// # Returns
    /// ResumableSyncResult containing the sync result and checkpoint (if sync failed)
    pub async fn execute_resumable<F>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SyncCheckpoint>,
        mut on_progress: F,
    ) -> DomainResult<ResumableSyncResult>
    where
        F: FnMut(&SyncCheckpoint) + Send,
    {
        if checkpoint.is_some() {
            info!("Resuming sync for project: {} from checkpoint", project_key);
        } else {
            info!("Syncing project: {}", project_key);
        }

        let started_at = Utc::now();
        let sync_type = if checkpoint.is_some() {
            "resumable"
        } else {
            "full"
        };
        let history_id = self
            .sync_history_repository
            .insert(project_id, sync_type, started_at)?;

        match self
            .sync_internal_resumable(project_key, project_id, checkpoint, &mut on_progress)
            .await
        {
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
                Ok(ResumableSyncResult {
                    sync_result: SyncResult::success(
                        project_key.to_string(),
                        issues_count,
                        history_count,
                    ),
                    checkpoint: None, // Clear checkpoint on success
                })
            }
            Err((e, last_checkpoint)) => {
                let completed_at = Utc::now();
                self.sync_history_repository.update_failed(
                    history_id,
                    &e.to_string(),
                    completed_at,
                )?;
                Ok(ResumableSyncResult {
                    sync_result: SyncResult::failure(project_key.to_string(), e.to_string()),
                    checkpoint: last_checkpoint, // Keep checkpoint for resume
                })
            }
        }
    }

    /// Internal sync with resumable support
    /// Returns Ok((issues_count, history_count)) on success
    /// Returns Err((error, last_checkpoint)) on failure with the last successful checkpoint
    async fn sync_internal_resumable<F>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SyncCheckpoint>,
        on_progress: &mut F,
    ) -> Result<(usize, usize), (crate::domain::error::DomainError, Option<SyncCheckpoint>)>
    where
        F: FnMut(&SyncCheckpoint) + Send,
    {
        info!("Fetching issues for project: {}", project_key);

        // Determine where to resume from
        let after_updated_at = checkpoint.as_ref().map(|cp| cp.last_issue_updated_at);
        let skip_until_key = checkpoint.as_ref().map(|cp| cp.last_issue_key.clone());
        let mut items_processed = checkpoint
            .as_ref()
            .map(|cp| cp.items_processed)
            .unwrap_or(0);

        // Collect all issues using batch fetching
        let mut all_issues: Vec<Issue> = Vec::new();
        let mut all_issue_keys: Vec<String> = Vec::new();
        let mut start_at = 0;
        let max_results = 100;
        let mut total_items: usize;
        let mut last_checkpoint: Option<SyncCheckpoint> = checkpoint.clone();
        let mut skipping = skip_until_key.is_some();

        loop {
            // Fetch a batch of issues
            let progress = self
                .jira_service
                .fetch_project_issues_batch(project_key, after_updated_at, start_at, max_results)
                .await
                .map_err(|e| (e, last_checkpoint.clone()))?;

            total_items = progress.total;

            if progress.issues.is_empty() {
                break;
            }

            // Filter out already processed issues when resuming
            let issues_to_process: Vec<Issue> = if skipping {
                let mut filtered = Vec::new();
                for issue in progress.issues {
                    if skipping {
                        // Skip until we find the issue after the checkpoint
                        if let Some(ref skip_key) = skip_until_key {
                            if issue.key == *skip_key {
                                skipping = false;
                                // Skip this issue too (it was already processed)
                                continue;
                            }
                        }
                        continue;
                    }
                    filtered.push(issue);
                }
                filtered
            } else {
                progress.issues
            };

            if !issues_to_process.is_empty() {
                info!(
                    "Processing batch: {} issues (total progress: {}/{})",
                    issues_to_process.len(),
                    items_processed + issues_to_process.len(),
                    total_items
                );

                // Save issues to database
                self.issue_repository
                    .batch_insert(&issues_to_process)
                    .map_err(|e| (e, last_checkpoint.clone()))?;

                // Save raw data to separate database if configured
                if let Some(ref raw_repo) = self.raw_repository {
                    let raw_data_items: Vec<(String, String, String, String)> = issues_to_process
                        .iter()
                        .filter_map(|issue| {
                            issue.raw_json.as_ref().map(|raw| {
                                (
                                    issue.id.clone(),
                                    issue.key.clone(),
                                    project_id.to_string(),
                                    raw.clone(),
                                )
                            })
                        })
                        .collect();

                    if !raw_data_items.is_empty() {
                        raw_repo
                            .batch_upsert_issue_raw_data(&raw_data_items)
                            .map_err(|e| (e, last_checkpoint.clone()))?;
                    }
                }

                // Extract and save change history for this batch
                for issue in &issues_to_process {
                    if let Some(raw_json) = &issue.raw_json {
                        self.change_history_repository
                            .delete_by_issue_id(&issue.id)
                            .map_err(|e| (e, last_checkpoint.clone()))?;

                        let history_items = ChangeHistoryItem::extract_from_raw_json(
                            &issue.id, &issue.key, raw_json,
                        );

                        if !history_items.is_empty() {
                            self.change_history_repository
                                .batch_insert(&history_items)
                                .map_err(|e| (e, last_checkpoint.clone()))?;
                        }
                    }
                }

                // Update checkpoint after successful batch processing
                let batch_len = issues_to_process.len();
                if batch_len > 0 {
                    items_processed += batch_len;
                    all_issue_keys.extend(issues_to_process.iter().map(|i| i.key.clone()));
                    all_issues.extend(issues_to_process);

                    // Get the last issue from all_issues (which now contains the batch)
                    if let Some(last_issue) = all_issues.last() {
                        let new_checkpoint = SyncCheckpoint {
                            last_issue_updated_at: last_issue.updated_date.unwrap_or_else(Utc::now),
                            last_issue_key: last_issue.key.clone(),
                            items_processed,
                            total_items,
                        };

                        // Notify progress callback
                        on_progress(&new_checkpoint);
                        last_checkpoint = Some(new_checkpoint);
                    }
                }
            }

            if !progress.has_more {
                break;
            }
            start_at = progress.fetched_so_far;
        }

        let count = all_issues.len();
        info!("Fetched and saved {} issues total", count);

        // Mark issues that no longer exist in JIRA as deleted (soft delete)
        // Only do this for full sync (not resumable)
        if checkpoint.is_none() && !all_issue_keys.is_empty() {
            let deleted_count = self
                .issue_repository
                .mark_deleted_not_in_keys(project_id, &all_issue_keys)
                .map_err(|e| (e, last_checkpoint.clone()))?;
            if deleted_count > 0 {
                info!(
                    "Marked {} issues as deleted (no longer in JIRA)",
                    deleted_count
                );
            }
        }

        // Count total history items
        let total_history_items = all_issues
            .iter()
            .filter_map(|i| i.raw_json.as_ref())
            .map(|json| ChangeHistoryItem::extract_from_raw_json("", "", json).len())
            .sum();

        if total_history_items > 0 {
            info!("Saved {} change history items", total_history_items);
        }

        // Fetch and save metadata
        self.sync_metadata(project_key, project_id)
            .await
            .map_err(|e| (e, last_checkpoint.clone()))?;

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
