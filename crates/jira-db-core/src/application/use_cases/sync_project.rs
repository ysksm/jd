use crate::application::dto::SyncResult;
use crate::application::services::JiraService;
use crate::application::use_cases::GenerateSnapshotsUseCase;
use crate::application::use_cases::generate_snapshots::create_snapshot_checkpoint;
use crate::application::use_cases::sync_logger::{SyncLogger, SyncSummaryReport};
use crate::domain::entities::{ChangeHistoryItem, Issue};
use crate::domain::error::DomainResult;
use crate::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository, MetadataRepository,
    SyncHistoryRepository,
};
use crate::infrastructure::config::{SnapshotCheckpoint, SyncCheckpoint};
use crate::infrastructure::database::SharedRawDataRepository;
use chrono::{DateTime, Utc};
use log::warn;
use std::sync::Arc;

/// Result of resumable sync operation
#[derive(Debug)]
pub struct ResumableSyncResult {
    pub sync_result: SyncResult,
    /// Checkpoint for issue sync (Some if issue sync failed and can be resumed, None if completed)
    pub checkpoint: Option<SyncCheckpoint>,
    /// Checkpoint for snapshot generation (Some if snapshot generation failed and can be resumed)
    pub snapshot_checkpoint: Option<SnapshotCheckpoint>,
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

    /// Execute sync for a project
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `project_id` - The JIRA project ID
    /// * `after_updated_at` - Only fetch issues updated at or after this timestamp (for incremental sync)
    ///
    /// # Deprecated
    /// Use `execute_resumable()` instead for checkpoint support and interruption safety.
    /// This method does not save progress during sync, so if interrupted, sync will restart from the beginning.
    #[deprecated(
        since = "0.1.0",
        note = "Use execute_resumable() instead for checkpoint support and interruption safety"
    )]
    pub async fn execute(
        &self,
        project_key: &str,
        project_id: &str,
        after_updated_at: Option<DateTime<Utc>>,
    ) -> DomainResult<SyncResult> {
        // Create a checkpoint from after_updated_at if provided
        let checkpoint = after_updated_at.map(|ts| SyncCheckpoint {
            last_issue_updated_at: ts,
            last_issue_key: String::new(), // No specific key to skip
            items_processed: 0,
            total_items: 0,
        });

        let result = self
            .execute_resumable(project_key, project_id, checkpoint, |_| {})
            .await?;
        Ok(result.sync_result)
    }

    /// Execute resumable sync with checkpoint support
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `project_id` - The JIRA project ID
    /// * `checkpoint` - Optional checkpoint to resume issue sync from
    /// * `on_progress` - Callback called after each batch with the new checkpoint
    ///
    /// # Returns
    /// ResumableSyncResult containing the sync result and checkpoint (if sync failed)
    pub async fn execute_resumable<F>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SyncCheckpoint>,
        on_progress: F,
    ) -> DomainResult<ResumableSyncResult>
    where
        F: FnMut(&SyncCheckpoint) + Send,
    {
        self.execute_resumable_with_snapshot_checkpoint(
            project_key,
            project_id,
            checkpoint,
            None, // No snapshot checkpoint
            on_progress,
            |_| {}, // No-op for snapshot progress
        )
        .await
    }

    /// Execute resumable sync with both issue and snapshot checkpoint support
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `project_id` - The JIRA project ID
    /// * `checkpoint` - Optional checkpoint to resume issue sync from
    /// * `snapshot_checkpoint` - Optional checkpoint to resume snapshot generation from
    ///   (if provided, issue sync is skipped and only snapshot generation is performed)
    /// * `on_progress` - Callback called after each issue batch with the new checkpoint
    /// * `on_snapshot_progress` - Callback called after each snapshot batch with the checkpoint
    ///
    /// # Returns
    /// ResumableSyncResult containing the sync result and checkpoints (if sync failed)
    pub async fn execute_resumable_with_snapshot_checkpoint<F, G>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SyncCheckpoint>,
        snapshot_checkpoint: Option<SnapshotCheckpoint>,
        mut on_progress: F,
        mut on_snapshot_progress: G,
    ) -> DomainResult<ResumableSyncResult>
    where
        F: FnMut(&SyncCheckpoint) + Send,
        G: FnMut(&SnapshotCheckpoint) + Send,
    {
        let started_at = Utc::now();

        // If we have a snapshot checkpoint, skip issue sync and go directly to snapshot generation
        if snapshot_checkpoint.is_some() {
            return self
                .execute_snapshot_only(
                    project_key,
                    project_id,
                    snapshot_checkpoint,
                    on_snapshot_progress,
                )
                .await;
        }

        let sync_type = if checkpoint.is_some() {
            "resumable"
        } else {
            "full"
        };
        let history_id = self
            .sync_history_repository
            .insert(project_id, sync_type, started_at)?;

        match self
            .sync_internal_resumable(
                project_key,
                project_id,
                checkpoint,
                &mut on_progress,
                &mut on_snapshot_progress,
            )
            .await
        {
            Ok((issues_count, history_count, last_issue_updated_at, snapshot_cp)) => {
                let completed_at = Utc::now();
                self.sync_history_repository.update_completed(
                    history_id,
                    issues_count,
                    completed_at,
                )?;

                Ok(ResumableSyncResult {
                    sync_result: SyncResult::success(
                        project_key.to_string(),
                        issues_count,
                        history_count,
                        last_issue_updated_at,
                    ),
                    checkpoint: None,                 // Clear checkpoint on success
                    snapshot_checkpoint: snapshot_cp, // May have snapshot checkpoint if snapshot generation failed
                })
            }
            Err((e, last_checkpoint, snapshot_cp)) => {
                let completed_at = Utc::now();
                self.sync_history_repository.update_failed(
                    history_id,
                    &e.to_string(),
                    completed_at,
                )?;
                Ok(ResumableSyncResult {
                    sync_result: SyncResult::failure(project_key.to_string(), e.to_string()),
                    checkpoint: last_checkpoint, // Keep checkpoint for resume
                    snapshot_checkpoint: snapshot_cp, // May have snapshot checkpoint
                })
            }
        }
    }

    /// Execute only snapshot generation with checkpoint support
    /// This is used when resuming from a snapshot checkpoint
    async fn execute_snapshot_only<G>(
        &self,
        project_key: &str,
        project_id: &str,
        snapshot_checkpoint: Option<SnapshotCheckpoint>,
        mut on_snapshot_progress: G,
    ) -> DomainResult<ResumableSyncResult>
    where
        G: FnMut(&SnapshotCheckpoint) + Send,
    {
        let mut logger = SyncLogger::new(project_key, 2);
        logger.start();

        // Step 1: Sync metadata (always re-fetch)
        let step1 = logger.step("Syncing project metadata");
        if let Err(e) = self.sync_metadata(project_key, project_id, &step1).await {
            warn!("Failed to sync metadata: {}", e);
            step1.detail(&format!("Warning: {}", e));
        }
        step1.finish();

        // Step 2: Generate issue snapshots
        let step2 = logger.step("Generating issue snapshots (resuming)");
        let snapshot_use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&self.issue_repository),
            Arc::clone(&self.change_history_repository),
            Arc::clone(&self.snapshot_repository),
        );

        let step2_ref = &step2;
        let mut last_snapshot_cp: Option<SnapshotCheckpoint> = None;
        let snapshot_result = snapshot_use_case.execute_with_progress(
            project_key,
            project_id,
            snapshot_checkpoint.clone(),
            |progress| {
                step2_ref.detail(&format!(
                    "Processing: {}/{} issues ({} snapshots)",
                    progress.issues_processed, progress.total_issues, progress.snapshots_generated
                ));
                // Save checkpoint after each batch for continuous progress tracking
                if progress.issues_processed > 0 {
                    let cp = create_snapshot_checkpoint(
                        &progress.last_issue_id,
                        &progress.last_issue_key,
                        progress.issues_processed,
                        progress.total_issues,
                        progress.snapshots_generated,
                    );
                    on_snapshot_progress(&cp);
                    last_snapshot_cp = Some(cp);
                }
            },
        );

        match snapshot_result {
            Ok(result) => {
                step2.detail(&format!(
                    "Generated {} snapshots for {} issues",
                    result.snapshots_generated, result.issues_processed
                ));
                step2.finish();

                // Get issue count for the result
                let issues_count = self.issue_repository.count_by_project(project_id)?;

                Ok(ResumableSyncResult {
                    sync_result: SyncResult::success(
                        project_key.to_string(),
                        issues_count,
                        0,
                        None,
                    ),
                    checkpoint: None,
                    snapshot_checkpoint: None, // Clear on success
                })
            }
            Err(e) => {
                step2.detail(&format!("Failed: {}", e));
                step2.finish();

                // Return with checkpoint for resume
                Ok(ResumableSyncResult {
                    sync_result: SyncResult::failure(project_key.to_string(), e.to_string()),
                    checkpoint: None,
                    snapshot_checkpoint, // Keep checkpoint for resume
                })
            }
        }
    }

    /// Internal sync with resumable support
    /// Returns Ok((issues_count, history_count, last_issue_updated_at, snapshot_checkpoint)) on success
    /// Returns Err((error, last_checkpoint, snapshot_checkpoint)) on failure with the last successful checkpoint
    async fn sync_internal_resumable<F, G>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SyncCheckpoint>,
        on_progress: &mut F,
        on_snapshot_progress: &mut G,
    ) -> Result<
        (
            usize,
            usize,
            Option<DateTime<Utc>>,
            Option<SnapshotCheckpoint>,
        ),
        (
            crate::domain::error::DomainError,
            Option<SyncCheckpoint>,
            Option<SnapshotCheckpoint>,
        ),
    >
    where
        F: FnMut(&SyncCheckpoint) + Send,
        G: FnMut(&SnapshotCheckpoint) + Send,
    {
        // Create logger with 4 main steps
        let mut logger = SyncLogger::new(project_key, 4);
        logger.start();

        // Step 1: Fetch issues from JIRA
        let step1 = logger.step("Fetching issues from JIRA");

        // Determine where to resume from
        let after_updated_at = checkpoint.as_ref().map(|cp| cp.last_issue_updated_at);
        // Only skip if we have a specific key to skip to (not empty string)
        let skip_until_key = checkpoint
            .as_ref()
            .map(|cp| cp.last_issue_key.clone())
            .filter(|key| !key.is_empty());
        let mut items_processed = checkpoint
            .as_ref()
            .map(|cp| cp.items_processed)
            .unwrap_or(0);

        if let Some(ref ts) = after_updated_at {
            step1.detail(&format!("Incremental sync from: {}", ts));
        } else {
            step1.detail("Full sync (no checkpoint)");
        }

        // Collect all issues using batch fetching with token-based pagination
        let mut all_issues: Vec<Issue> = Vec::new();
        let mut all_issue_keys: Vec<String> = Vec::new();
        let mut page_token: Option<String> = None;
        let max_results = 100;
        let mut last_checkpoint: Option<SyncCheckpoint> = checkpoint.clone();
        let mut skipping = skip_until_key.is_some();
        let mut batch_count = 0;

        loop {
            batch_count += 1;
            step1.detail(&format!(
                "Batch {}: page_token={:?}",
                batch_count,
                page_token.as_ref().map(|t| &t[..t.len().min(20)])
            ));

            // Fetch a batch of issues
            let progress = self
                .jira_service
                .fetch_project_issues_batch(
                    project_key,
                    after_updated_at,
                    page_token.as_deref(),
                    max_results,
                )
                .await
                .map_err(|e| (e, last_checkpoint.clone(), None))?;

            step1.detail(&format!(
                "  -> Fetched {} issues, has_more={}",
                progress.issues.len(),
                progress.has_more
            ));

            if progress.issues.is_empty() {
                step1.detail("  -> Empty batch, stopping pagination");
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
                step1.detail(&format!(
                    "  -> Processing {} issues (total so far: {})",
                    issues_to_process.len(),
                    items_processed + issues_to_process.len()
                ));

                // Save issues to database
                self.issue_repository
                    .batch_insert(&issues_to_process)
                    .map_err(|e| (e, last_checkpoint.clone(), None))?;

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
                            .map_err(|e| (e, last_checkpoint.clone(), None))?;
                    }
                }

                // Extract and save change history for this batch
                for issue in &issues_to_process {
                    if let Some(raw_json) = &issue.raw_json {
                        self.change_history_repository
                            .delete_by_issue_id(&issue.id)
                            .map_err(|e| (e, last_checkpoint.clone(), None))?;

                        let history_items = ChangeHistoryItem::extract_from_raw_json(
                            &issue.id, &issue.key, raw_json,
                        );

                        if !history_items.is_empty() {
                            self.change_history_repository
                                .batch_insert(&history_items)
                                .map_err(|e| (e, last_checkpoint.clone(), None))?;
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
                        let checkpoint_time = last_issue.updated_date.unwrap_or_else(|| {
                            log::warn!(
                                "[SyncProject] Issue {} has no updated_date, falling back to Utc::now()",
                                last_issue.key
                            );
                            Utc::now()
                        });
                        log::debug!(
                            "[SyncProject] Creating checkpoint: issue={}, updated_date={:?}, checkpoint_time={}",
                            last_issue.key,
                            last_issue.updated_date,
                            checkpoint_time
                        );
                        let new_checkpoint = SyncCheckpoint {
                            last_issue_updated_at: checkpoint_time,
                            last_issue_key: last_issue.key.clone(),
                            items_processed,
                            total_items: all_issues.len(),
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

            // Update page token for next iteration
            page_token = progress.next_page_token;
        }

        let count = all_issues.len();

        // Mark issues that no longer exist in JIRA as deleted (soft delete)
        // Only do this for full sync (not resumable)
        let mut deleted_count = 0;
        if checkpoint.is_none() && !all_issue_keys.is_empty() {
            deleted_count = self
                .issue_repository
                .mark_deleted_not_in_keys(project_id, &all_issue_keys)
                .map_err(|e| (e, last_checkpoint.clone(), None))?;
        }

        // Count total history items
        let total_history_items: usize = all_issues
            .iter()
            .filter_map(|i| i.raw_json.as_ref())
            .map(|json| ChangeHistoryItem::extract_from_raw_json("", "", json).len())
            .sum();

        step1.finish_with_detail(&format!(
            "Saved {} issues, {} change history items{}",
            count,
            total_history_items,
            if deleted_count > 0 {
                format!(", {} deleted", deleted_count)
            } else {
                String::new()
            }
        ));

        // Step 2: Sync metadata
        let step2 = logger.step("Syncing project metadata");
        self.sync_metadata(project_key, project_id, &step2)
            .await
            .map_err(|e| (e, last_checkpoint.clone(), None))?;
        step2.finish();

        // Step 3: Generate issue snapshots (with batch processing for large datasets)
        let step3 = logger.step("Generating issue snapshots");
        let snapshot_use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&self.issue_repository),
            Arc::clone(&self.change_history_repository),
            Arc::clone(&self.snapshot_repository),
        );

        // Track snapshot checkpoint for resume support
        let mut last_snapshot_checkpoint: Option<SnapshotCheckpoint> = None;

        // Use progress callback to show batch progress and save checkpoint continuously
        let step3_ref = &step3;
        let (snapshot_count, snapshot_failed) = match snapshot_use_case.execute_with_progress(
            project_key,
            project_id,
            None, // Fresh generation (no checkpoint yet)
            |progress| {
                step3_ref.detail(&format!(
                    "Processing: {}/{} issues ({} snapshots)",
                    progress.issues_processed, progress.total_issues, progress.snapshots_generated
                ));
                // Save checkpoint after each batch for continuous progress tracking
                if progress.issues_processed > 0 {
                    let cp = create_snapshot_checkpoint(
                        &progress.last_issue_id,
                        &progress.last_issue_key,
                        progress.issues_processed,
                        progress.total_issues,
                        progress.snapshots_generated,
                    );
                    on_snapshot_progress(&cp);
                    last_snapshot_checkpoint = Some(cp);
                }
            },
        ) {
            Ok(result) => {
                step3.detail(&format!(
                    "Generated {} snapshots for {} issues",
                    result.snapshots_generated, result.issues_processed
                ));
                last_snapshot_checkpoint = None; // Clear on success
                (result.snapshots_generated, false)
            }
            Err(e) => {
                warn!("Failed to generate snapshots: {}", e);
                step3.detail(&format!("Error: {} (can be resumed)", e));
                // Keep last_snapshot_checkpoint for resume
                (0, true)
            }
        };
        step3.finish();

        // Step 4: Verify data integrity
        let step4 = logger.step("Verifying data integrity");
        let mut summary = SyncSummaryReport::default();
        summary.issues_synced = count;
        summary.success = true;

        // Get JIRA total count (most reliable method)
        step4.detail("Fetching JIRA total issue count...");
        match self.jira_service.get_total_issue_count(project_key).await {
            Ok(total) => {
                summary.jira_total_count = total;
                step4.detail(&format!("JIRA total issue count: {}", total));
            }
            Err(e) => {
                step4.detail(&format!("Warning: Could not fetch JIRA total count: {}", e));
            }
        }

        // Get local counts by status
        step4.detail("Fetching local issue counts by status...");
        match self.issue_repository.count_by_status(project_id) {
            Ok(local_counts) => {
                summary.local_status_counts = local_counts.clone();
                summary.local_total_count = local_counts.values().sum();
            }
            Err(e) => {
                step4.detail(&format!("Warning: Could not fetch local counts: {}", e));
            }
        }

        // Get local history and snapshot counts
        summary.local_history_count = total_history_items;
        summary.local_snapshot_count = snapshot_count;

        // Get the last issue's updated_date for incremental sync
        // Use the checkpoint's value which is guaranteed to be set (with Utc::now() fallback)
        let last_issue_updated_at = last_checkpoint.as_ref().map(|cp| cp.last_issue_updated_at);
        summary.last_issue_updated_at = last_issue_updated_at;

        step4.finish();

        // Output summary
        logger.summary(&summary);

        // Return result with snapshot checkpoint if snapshot generation failed
        // (so it can be resumed without re-fetching issues)
        Ok((
            count,
            total_history_items,
            last_issue_updated_at,
            if snapshot_failed {
                last_snapshot_checkpoint
            } else {
                None
            },
        ))
    }

    async fn sync_metadata(
        &self,
        project_key: &str,
        project_id: &str,
        step: &crate::application::use_cases::sync_logger::StepLogger,
    ) -> DomainResult<()> {
        // Fetch statuses
        match self.jira_service.fetch_project_statuses(project_key).await {
            Ok(statuses) => {
                if !statuses.is_empty() {
                    self.metadata_repository
                        .upsert_statuses(project_id, &statuses)?;
                    step.detail(&format!("Saved {} statuses", statuses.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch statuses: {}", e);
                step.detail(&format!("Warning: Failed to fetch statuses: {}", e));
            }
        }

        // Fetch priorities
        match self.jira_service.fetch_priorities().await {
            Ok(priorities) => {
                if !priorities.is_empty() {
                    self.metadata_repository
                        .upsert_priorities(project_id, &priorities)?;
                    step.detail(&format!("Saved {} priorities", priorities.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch priorities: {}", e);
                step.detail(&format!("Warning: Failed to fetch priorities: {}", e));
            }
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
                    step.detail(&format!("Saved {} issue types", issue_types.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch issue types: {}", e);
                step.detail(&format!("Warning: Failed to fetch issue types: {}", e));
            }
        }

        // Fetch labels
        match self.jira_service.fetch_project_labels(project_key).await {
            Ok(labels) => {
                if !labels.is_empty() {
                    self.metadata_repository
                        .upsert_labels(project_id, &labels)?;
                    step.detail(&format!("Saved {} labels", labels.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch labels: {}", e);
                step.detail(&format!("Warning: Failed to fetch labels: {}", e));
            }
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
                    step.detail(&format!("Saved {} components", components.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch components: {}", e);
                step.detail(&format!("Warning: Failed to fetch components: {}", e));
            }
        }

        // Fetch versions
        match self.jira_service.fetch_project_versions(project_key).await {
            Ok(fix_versions) => {
                if !fix_versions.is_empty() {
                    self.metadata_repository
                        .upsert_fix_versions(project_id, &fix_versions)?;
                    step.detail(&format!("Saved {} fix versions", fix_versions.len()));
                }
            }
            Err(e) => {
                warn!("Failed to fetch versions: {}", e);
                step.detail(&format!("Warning: Failed to fetch versions: {}", e));
            }
        }

        Ok(())
    }
}
