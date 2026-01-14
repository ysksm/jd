//! Tests for sync functionality including:
//! - Checkpoint/resumable sync (差分同期)
//! - Incremental sync (増分同期)
//! - Snapshot generation

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use crate::application::dto::{CreatedIssueDto, TransitionDto};
use crate::application::services::{FetchProgress, JiraService};
use crate::application::use_cases::{GenerateSnapshotsUseCase, SyncProjectUseCase};
use crate::domain::entities::{
    ChangeHistoryItem, Component, FixVersion, Issue, IssueSnapshot, IssueType, JiraField, Label,
    Priority, Project, Status,
};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::{
    ChangeHistoryRepository, IssuePage, IssueRepository, IssueSnapshotRepository,
    MetadataRepository, SearchParams, SyncHistoryRepository,
};
use crate::infrastructure::config::SyncCheckpoint;

// ============================================================================
// Mock Repositories
// ============================================================================

/// Mock Issue Repository that stores issues in memory
#[derive(Default)]
pub struct MockIssueRepository {
    issues: Mutex<Vec<Issue>>,
    deleted_keys: Mutex<Vec<String>>,
}

impl MockIssueRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_issues(&self) -> Vec<Issue> {
        self.issues.lock().unwrap().clone()
    }

    pub fn get_deleted_keys(&self) -> Vec<String> {
        self.deleted_keys.lock().unwrap().clone()
    }
}

impl IssueRepository for MockIssueRepository {
    fn batch_insert(&self, issues: &[Issue]) -> DomainResult<()> {
        let mut stored = self.issues.lock().unwrap();
        for issue in issues {
            // Update if exists, insert if not
            if let Some(pos) = stored.iter().position(|i| i.id == issue.id) {
                stored[pos] = issue.clone();
            } else {
                stored.push(issue.clone());
            }
        }
        Ok(())
    }

    fn find_by_project(&self, project_id: &str) -> DomainResult<Vec<Issue>> {
        let stored = self.issues.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|i| i.project_id == project_id)
            .cloned()
            .collect())
    }

    fn count_by_project(&self, project_id: &str) -> DomainResult<usize> {
        let stored = self.issues.lock().unwrap();
        Ok(stored.iter().filter(|i| i.project_id == project_id).count())
    }

    fn search(&self, _params: &SearchParams) -> DomainResult<Vec<Issue>> {
        Ok(self.issues.lock().unwrap().clone())
    }

    fn mark_deleted_not_in_keys(&self, _project_id: &str, keys: &[String]) -> DomainResult<usize> {
        let stored = self.issues.lock().unwrap();
        let mut deleted = self.deleted_keys.lock().unwrap();
        let mut count = 0;
        for issue in stored.iter() {
            if !keys.contains(&issue.key) {
                deleted.push(issue.key.clone());
                count += 1;
            }
        }
        Ok(count)
    }

    fn count_by_status(&self, _project_id: &str) -> DomainResult<HashMap<String, usize>> {
        let stored = self.issues.lock().unwrap();
        let mut counts = HashMap::new();
        for issue in stored.iter() {
            if let Some(status) = &issue.status {
                *counts.entry(status.clone()).or_insert(0) += 1;
            }
        }
        Ok(counts)
    }

    fn find_by_project_paginated(
        &self,
        project_id: &str,
        offset: usize,
        limit: usize,
    ) -> DomainResult<IssuePage> {
        let stored = self.issues.lock().unwrap();
        let filtered: Vec<_> = stored
            .iter()
            .filter(|i| i.project_id == project_id)
            .cloned()
            .collect();
        let total_count = filtered.len();
        let issues: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();
        let has_more = offset + issues.len() < total_count;
        Ok(IssuePage {
            issues,
            total_count,
            has_more,
        })
    }

    fn find_by_project_after_id(
        &self,
        project_id: &str,
        after_issue_id: &str,
        limit: usize,
    ) -> DomainResult<IssuePage> {
        let stored = self.issues.lock().unwrap();
        let filtered: Vec<_> = stored
            .iter()
            .filter(|i| i.project_id == project_id && i.id > after_issue_id.to_string())
            .cloned()
            .collect();
        let total_count = stored.iter().filter(|i| i.project_id == project_id).count();
        let issues: Vec<_> = filtered.into_iter().take(limit).collect();
        let has_more = issues.len() == limit;
        Ok(IssuePage {
            issues,
            total_count,
            has_more,
        })
    }
}

/// Mock Change History Repository
#[derive(Default)]
pub struct MockChangeHistoryRepository {
    history: Mutex<Vec<ChangeHistoryItem>>,
}

impl MockChangeHistoryRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_history(&self) -> Vec<ChangeHistoryItem> {
        self.history.lock().unwrap().clone()
    }
}

impl ChangeHistoryRepository for MockChangeHistoryRepository {
    fn batch_insert(&self, items: &[ChangeHistoryItem]) -> DomainResult<()> {
        let mut stored = self.history.lock().unwrap();
        stored.extend(items.iter().cloned());
        Ok(())
    }

    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()> {
        let mut stored = self.history.lock().unwrap();
        stored.retain(|h| h.issue_id != issue_id);
        Ok(())
    }

    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<ChangeHistoryItem>> {
        let stored = self.history.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|h| h.issue_key == issue_key)
            .cloned()
            .collect())
    }

    fn find_by_issue_key_and_field(
        &self,
        issue_key: &str,
        field_filter: Option<&str>,
    ) -> DomainResult<Vec<ChangeHistoryItem>> {
        let stored = self.history.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|h| h.issue_key == issue_key && field_filter.map_or(true, |f| h.field == f))
            .cloned()
            .collect())
    }

    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize> {
        let stored = self.history.lock().unwrap();
        Ok(stored.iter().filter(|h| h.issue_key == issue_key).count())
    }
}

/// Mock Metadata Repository
#[derive(Default)]
pub struct MockMetadataRepository {
    statuses: Mutex<Vec<Status>>,
    priorities: Mutex<Vec<Priority>>,
    issue_types: Mutex<Vec<IssueType>>,
    labels: Mutex<Vec<Label>>,
    components: Mutex<Vec<Component>>,
    fix_versions: Mutex<Vec<FixVersion>>,
}

impl MockMetadataRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MetadataRepository for MockMetadataRepository {
    fn upsert_statuses(&self, _project_id: &str, statuses: &[Status]) -> DomainResult<()> {
        let mut stored = self.statuses.lock().unwrap();
        *stored = statuses.to_vec();
        Ok(())
    }

    fn find_statuses_by_project(&self, _project_id: &str) -> DomainResult<Vec<Status>> {
        Ok(self.statuses.lock().unwrap().clone())
    }

    fn upsert_priorities(&self, _project_id: &str, priorities: &[Priority]) -> DomainResult<()> {
        let mut stored = self.priorities.lock().unwrap();
        *stored = priorities.to_vec();
        Ok(())
    }

    fn find_priorities_by_project(&self, _project_id: &str) -> DomainResult<Vec<Priority>> {
        Ok(self.priorities.lock().unwrap().clone())
    }

    fn upsert_issue_types(&self, _project_id: &str, issue_types: &[IssueType]) -> DomainResult<()> {
        let mut stored = self.issue_types.lock().unwrap();
        *stored = issue_types.to_vec();
        Ok(())
    }

    fn find_issue_types_by_project(&self, _project_id: &str) -> DomainResult<Vec<IssueType>> {
        Ok(self.issue_types.lock().unwrap().clone())
    }

    fn upsert_labels(&self, _project_id: &str, labels: &[Label]) -> DomainResult<()> {
        let mut stored = self.labels.lock().unwrap();
        *stored = labels.to_vec();
        Ok(())
    }

    fn find_labels_by_project(&self, _project_id: &str) -> DomainResult<Vec<Label>> {
        Ok(self.labels.lock().unwrap().clone())
    }

    fn upsert_components(&self, _project_id: &str, components: &[Component]) -> DomainResult<()> {
        let mut stored = self.components.lock().unwrap();
        *stored = components.to_vec();
        Ok(())
    }

    fn find_components_by_project(&self, _project_id: &str) -> DomainResult<Vec<Component>> {
        Ok(self.components.lock().unwrap().clone())
    }

    fn upsert_fix_versions(
        &self,
        _project_id: &str,
        fix_versions: &[FixVersion],
    ) -> DomainResult<()> {
        let mut stored = self.fix_versions.lock().unwrap();
        *stored = fix_versions.to_vec();
        Ok(())
    }

    fn find_fix_versions_by_project(&self, _project_id: &str) -> DomainResult<Vec<FixVersion>> {
        Ok(self.fix_versions.lock().unwrap().clone())
    }
}

/// Mock Sync History Repository
#[derive(Default)]
pub struct MockSyncHistoryRepository {
    next_id: Mutex<i64>,
}

impl MockSyncHistoryRepository {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
        }
    }
}

impl SyncHistoryRepository for MockSyncHistoryRepository {
    fn insert(
        &self,
        _project_id: &str,
        _sync_type: &str,
        _started_at: DateTime<Utc>,
    ) -> DomainResult<i64> {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        Ok(current)
    }

    fn update_completed(
        &self,
        _id: i64,
        _items_synced: usize,
        _completed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        Ok(())
    }

    fn update_failed(
        &self,
        _id: i64,
        _error_message: &str,
        _completed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        Ok(())
    }

    fn find_latest_by_project(
        &self,
        _project_id: &str,
    ) -> DomainResult<Option<(DateTime<Utc>, String)>> {
        Ok(None)
    }
}

/// Mock Issue Snapshot Repository
#[derive(Default)]
pub struct MockIssueSnapshotRepository {
    snapshots: Mutex<Vec<IssueSnapshot>>,
    in_transaction: Mutex<bool>,
}

impl MockIssueSnapshotRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_snapshots(&self) -> Vec<IssueSnapshot> {
        self.snapshots.lock().unwrap().clone()
    }
}

impl IssueSnapshotRepository for MockIssueSnapshotRepository {
    fn batch_insert(&self, snapshots: &[IssueSnapshot]) -> DomainResult<()> {
        let mut stored = self.snapshots.lock().unwrap();
        stored.extend(snapshots.iter().cloned());
        Ok(())
    }

    fn bulk_insert(&self, snapshots: &[IssueSnapshot]) -> DomainResult<()> {
        self.batch_insert(snapshots)
    }

    fn delete_by_issue_id(&self, issue_id: &str) -> DomainResult<()> {
        let mut stored = self.snapshots.lock().unwrap();
        stored.retain(|s| s.issue_id != issue_id);
        Ok(())
    }

    fn delete_by_project_id(&self, project_id: &str) -> DomainResult<()> {
        let mut stored = self.snapshots.lock().unwrap();
        stored.retain(|s| s.project_id != project_id);
        Ok(())
    }

    fn find_by_issue_key(&self, issue_key: &str) -> DomainResult<Vec<IssueSnapshot>> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|s| s.issue_key == issue_key)
            .cloned()
            .collect())
    }

    fn find_by_issue_key_and_version(
        &self,
        issue_key: &str,
        version: i32,
    ) -> DomainResult<Option<IssueSnapshot>> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored
            .iter()
            .find(|s| s.issue_key == issue_key && s.version == version)
            .cloned())
    }

    fn find_current_by_issue_key(&self, issue_key: &str) -> DomainResult<Option<IssueSnapshot>> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|s| s.issue_key == issue_key)
            .max_by_key(|s| s.version)
            .cloned())
    }

    fn find_by_project_id(&self, project_id: &str) -> DomainResult<Vec<IssueSnapshot>> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|s| s.project_id == project_id)
            .cloned()
            .collect())
    }

    fn count_by_issue_key(&self, issue_key: &str) -> DomainResult<usize> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored.iter().filter(|s| s.issue_key == issue_key).count())
    }

    fn count_by_project_id(&self, project_id: &str) -> DomainResult<usize> {
        let stored = self.snapshots.lock().unwrap();
        Ok(stored.iter().filter(|s| s.project_id == project_id).count())
    }

    fn begin_transaction(&self) -> DomainResult<()> {
        *self.in_transaction.lock().unwrap() = true;
        Ok(())
    }

    fn commit_transaction(&self) -> DomainResult<()> {
        *self.in_transaction.lock().unwrap() = false;
        Ok(())
    }

    fn rollback_transaction(&self) -> DomainResult<()> {
        *self.in_transaction.lock().unwrap() = false;
        Ok(())
    }
}

/// Mock JIRA Service for testing
pub struct MockJiraService {
    /// Issues to return, organized by batch
    issues_batches: Mutex<Vec<Vec<Issue>>>,
    /// Current batch index
    current_batch: Mutex<usize>,
    /// Track calls for verification
    fetch_calls: Mutex<Vec<FetchCall>>,
    /// Total issue count
    total_count: usize,
    /// Simulated failure at batch index (None = no failure)
    fail_at_batch: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FetchCall {
    pub project_key: String,
    pub after_updated_at: Option<DateTime<Utc>>,
    pub page_token: Option<String>,
}

impl MockJiraService {
    pub fn new(issues_batches: Vec<Vec<Issue>>) -> Self {
        let total_count = issues_batches.iter().map(|b| b.len()).sum();
        Self {
            issues_batches: Mutex::new(issues_batches),
            current_batch: Mutex::new(0),
            fetch_calls: Mutex::new(Vec::new()),
            total_count,
            fail_at_batch: None,
        }
    }

    pub fn with_failure_at_batch(mut self, batch_index: usize) -> Self {
        self.fail_at_batch = Some(batch_index);
        self
    }

    pub fn get_fetch_calls(&self) -> Vec<FetchCall> {
        self.fetch_calls.lock().unwrap().clone()
    }

    pub fn reset(&self) {
        *self.current_batch.lock().unwrap() = 0;
        self.fetch_calls.lock().unwrap().clear();
    }
}

#[async_trait]
impl JiraService for MockJiraService {
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>> {
        Ok(vec![])
    }

    async fn fetch_project_issues(&self, _project_key: &str) -> DomainResult<Vec<Issue>> {
        let batches = self.issues_batches.lock().unwrap();
        Ok(batches.iter().flatten().cloned().collect())
    }

    async fn fetch_project_issues_batch(
        &self,
        project_key: &str,
        after_updated_at: Option<DateTime<Utc>>,
        page_token: Option<&str>,
        _max_results: usize,
    ) -> DomainResult<FetchProgress> {
        // Record this call
        self.fetch_calls.lock().unwrap().push(FetchCall {
            project_key: project_key.to_string(),
            after_updated_at,
            page_token: page_token.map(String::from),
        });

        let mut batch_idx = self.current_batch.lock().unwrap();
        let batches = self.issues_batches.lock().unwrap();

        // Check for simulated failure
        if let Some(fail_at) = self.fail_at_batch {
            if *batch_idx == fail_at {
                return Err(DomainError::ExternalService(
                    "Simulated network failure".to_string(),
                ));
            }
        }

        if *batch_idx >= batches.len() {
            return Ok(FetchProgress {
                issues: vec![],
                total: self.total_count,
                fetched_so_far: self.total_count,
                has_more: false,
                next_page_token: None,
            });
        }

        let issues = batches[*batch_idx].clone();
        let has_more = *batch_idx + 1 < batches.len();
        let next_token = if has_more {
            Some(format!("token_{}", *batch_idx + 1))
        } else {
            None
        };

        *batch_idx += 1;

        Ok(FetchProgress {
            issues,
            total: self.total_count,
            fetched_so_far: batches[..*batch_idx].iter().map(|b| b.len()).sum(),
            has_more,
            next_page_token: next_token,
        })
    }

    async fn test_connection(&self) -> DomainResult<()> {
        Ok(())
    }

    async fn fetch_project_statuses(&self, _project_key: &str) -> DomainResult<Vec<Status>> {
        Ok(vec![])
    }

    async fn fetch_priorities(&self) -> DomainResult<Vec<Priority>> {
        Ok(vec![])
    }

    async fn fetch_project_issue_types(&self, _project_id: &str) -> DomainResult<Vec<IssueType>> {
        Ok(vec![])
    }

    async fn fetch_issue_types_by_project_key(
        &self,
        _project_key: &str,
    ) -> DomainResult<Vec<IssueType>> {
        Ok(vec![])
    }

    async fn fetch_project_labels(&self, _project_key: &str) -> DomainResult<Vec<Label>> {
        Ok(vec![])
    }

    async fn fetch_project_components(&self, _project_key: &str) -> DomainResult<Vec<Component>> {
        Ok(vec![])
    }

    async fn fetch_project_versions(&self, _project_key: &str) -> DomainResult<Vec<FixVersion>> {
        Ok(vec![])
    }

    async fn fetch_fields(&self) -> DomainResult<Vec<JiraField>> {
        Ok(vec![])
    }

    async fn create_issue(
        &self,
        _project_key: &str,
        _summary: &str,
        _description: Option<&str>,
        _issue_type: &str,
    ) -> DomainResult<CreatedIssueDto> {
        unimplemented!()
    }

    async fn get_issue_transitions(&self, _issue_key: &str) -> DomainResult<Vec<TransitionDto>> {
        Ok(vec![])
    }

    async fn transition_issue(&self, _issue_key: &str, _transition_id: &str) -> DomainResult<()> {
        Ok(())
    }

    async fn get_issue_count_by_status(
        &self,
        _project_key: &str,
    ) -> DomainResult<HashMap<String, usize>> {
        Ok(HashMap::new())
    }

    async fn get_total_issue_count(&self, _project_key: &str) -> DomainResult<usize> {
        Ok(self.total_count)
    }

    async fn create_issue_link(
        &self,
        _link_type: &str,
        _inward_issue: &str,
        _outward_issue: &str,
    ) -> DomainResult<()> {
        Ok(())
    }

    async fn update_issue_due_date(&self, _issue_key: &str, _due_date: &str) -> DomainResult<()> {
        Ok(())
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_issue(id: &str, key: &str, project_id: &str, updated_at: DateTime<Utc>) -> Issue {
    Issue::new(
        id.to_string(),
        project_id.to_string(),
        key.to_string(),
        format!("Test Issue {}", key),
        Some("Description".to_string()),
        Some("Open".to_string()),
        Some("High".to_string()),
        Some("user@example.com".to_string()),
        Some("reporter@example.com".to_string()),
        Some("Bug".to_string()),
        None,
        Some(vec!["label1".to_string()]),
        Some(vec!["component1".to_string()]),
        Some(vec!["1.0".to_string()]),
        None,
        None,
        None,
        None,
        Some(updated_at - Duration::days(1)),
        Some(updated_at),
        Some(r#"{"changelog":{"histories":[]}}"#.to_string()),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Full sync saves all issues (raw, flatten, change_history)
    #[tokio::test]
    async fn test_full_sync_saves_all_data_types() {
        let now = Utc::now();
        let issues = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
            create_test_issue("3", "PROJ-3", "100", now),
        ];

        let jira_service = Arc::new(MockJiraService::new(vec![issues]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        let result = use_case
            .execute_resumable("PROJ", "100", None, |_| {})
            .await
            .unwrap();

        // Verify sync result
        assert!(result.sync_result.success);
        assert_eq!(result.sync_result.issues_synced, 3);
        assert!(result.checkpoint.is_none()); // Cleared on success

        // Verify issues were saved (flatten data)
        let saved_issues = issue_repo.get_issues();
        assert_eq!(saved_issues.len(), 3);

        // Verify raw_json is preserved
        for issue in &saved_issues {
            assert!(issue.raw_json.is_some());
        }
    }

    /// Test: Checkpoint is saved after each batch
    #[tokio::test]
    async fn test_checkpoint_saved_after_each_batch() {
        let now = Utc::now();
        let batch1 = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
        ];
        let batch2 = vec![create_test_issue("3", "PROJ-3", "100", now)];

        let jira_service = Arc::new(MockJiraService::new(vec![batch1, batch2]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let checkpoints_clone = Arc::clone(&checkpoints);

        let result = use_case
            .execute_resumable("PROJ", "100", None, move |cp| {
                checkpoints_clone.lock().unwrap().push(cp.clone());
            })
            .await
            .unwrap();

        assert!(result.sync_result.success);

        // Verify checkpoints were recorded
        let saved_checkpoints = checkpoints.lock().unwrap();
        assert_eq!(saved_checkpoints.len(), 2); // One per batch

        // First checkpoint after batch 1
        assert_eq!(saved_checkpoints[0].items_processed, 2);
        assert_eq!(saved_checkpoints[0].last_issue_key, "PROJ-2");

        // Second checkpoint after batch 2
        assert_eq!(saved_checkpoints[1].items_processed, 3);
        assert_eq!(saved_checkpoints[1].last_issue_key, "PROJ-3");
    }

    /// Test: Resume from checkpoint skips already processed issues
    #[tokio::test]
    async fn test_resume_from_checkpoint() {
        let now = Utc::now();
        // Simulate batch 1 was already processed, resuming from batch 2
        let batch1 = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
        ];
        let batch2 = vec![create_test_issue("3", "PROJ-3", "100", now)];

        let jira_service = Arc::new(MockJiraService::new(vec![batch1, batch2]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        // Resume with checkpoint from batch 1
        let checkpoint = SyncCheckpoint {
            last_issue_updated_at: now - Duration::hours(1),
            last_issue_key: "PROJ-2".to_string(),
            items_processed: 2,
            total_items: 3,
        };

        let result = use_case
            .execute_resumable("PROJ", "100", Some(checkpoint), |_| {})
            .await
            .unwrap();

        assert!(result.sync_result.success);

        // Verify JIRA service was called with after_updated_at
        let calls = jira_service.get_fetch_calls();
        assert!(!calls.is_empty());
        assert!(calls[0].after_updated_at.is_some());
    }

    /// Test: Sync failure returns checkpoint for resume
    #[tokio::test]
    async fn test_sync_failure_returns_checkpoint() {
        let now = Utc::now();
        let batch1 = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
        ];
        let batch2 = vec![create_test_issue("3", "PROJ-3", "100", now)];

        // Fail at batch 2
        let jira_service =
            Arc::new(MockJiraService::new(vec![batch1, batch2]).with_failure_at_batch(1));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        let result = use_case
            .execute_resumable("PROJ", "100", None, |_| {})
            .await
            .unwrap();

        // Verify failure and checkpoint returned
        assert!(!result.sync_result.success);
        assert!(result.checkpoint.is_some());

        let checkpoint = result.checkpoint.unwrap();
        assert_eq!(checkpoint.items_processed, 2); // Batch 1 was processed
        assert_eq!(checkpoint.last_issue_key, "PROJ-2");
    }

    /// Test: Incremental sync only fetches updated issues
    #[tokio::test]
    async fn test_incremental_sync_with_after_updated_at() {
        let now = Utc::now();
        let issues = vec![create_test_issue("3", "PROJ-3", "100", now)];

        let jira_service = Arc::new(MockJiraService::new(vec![issues]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        // Simulate incremental sync from 1 hour ago
        let checkpoint = SyncCheckpoint {
            last_issue_updated_at: now - Duration::hours(1),
            last_issue_key: String::new(), // Empty means don't skip any
            items_processed: 0,
            total_items: 0,
        };

        let result = use_case
            .execute_resumable("PROJ", "100", Some(checkpoint), |_| {})
            .await
            .unwrap();

        assert!(result.sync_result.success);

        // Verify after_updated_at was passed to JIRA service
        let calls = jira_service.get_fetch_calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].after_updated_at.is_some());

        let after = calls[0].after_updated_at.unwrap();
        assert!(after <= now - Duration::minutes(55)); // ~1 hour ago
    }

    /// Test: Snapshot generation for issues without change history
    #[tokio::test]
    async fn test_snapshot_generation_no_history() {
        let now = Utc::now();
        let issues = vec![
            create_test_issue("1", "PROJ-1", "100", now),
            create_test_issue("2", "PROJ-2", "100", now),
        ];

        let issue_repo = Arc::new(MockIssueRepository::new());
        issue_repo.batch_insert(&issues).unwrap();

        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&snapshot_repo),
        );

        let result = use_case.execute("PROJ", "100").unwrap();

        assert!(result.completed);
        assert_eq!(result.issues_processed, 2);
        assert_eq!(result.snapshots_generated, 2); // One snapshot per issue (no history)

        // Verify snapshots were created
        let snapshots = snapshot_repo.get_snapshots();
        assert_eq!(snapshots.len(), 2);
    }

    /// Test: Snapshot generation with change history creates multiple versions
    #[tokio::test]
    async fn test_snapshot_generation_with_history() {
        let now = Utc::now();
        let issue = Issue::new(
            "1".to_string(),
            "100".to_string(),
            "PROJ-1".to_string(),
            "Test Issue".to_string(),
            None,
            Some("Done".to_string()),
            Some("High".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(now - Duration::days(2)),
            Some(now),
            None,
        );

        let issue_repo = Arc::new(MockIssueRepository::new());
        issue_repo.batch_insert(&[issue]).unwrap();

        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        // Add change history: Open -> In Progress -> Done
        let history = vec![
            ChangeHistoryItem {
                issue_id: "1".to_string(),
                issue_key: "PROJ-1".to_string(),
                history_id: "hist1".to_string(),
                author_account_id: Some("user1".to_string()),
                author_display_name: Some("User One".to_string()),
                field: "status".to_string(),
                field_type: Some("jira".to_string()),
                from_value: None,
                from_string: Some("Open".to_string()),
                to_value: None,
                to_string: Some("In Progress".to_string()),
                changed_at: now - Duration::days(1),
            },
            ChangeHistoryItem {
                issue_id: "1".to_string(),
                issue_key: "PROJ-1".to_string(),
                history_id: "hist2".to_string(),
                author_account_id: Some("user1".to_string()),
                author_display_name: Some("User One".to_string()),
                field: "status".to_string(),
                field_type: Some("jira".to_string()),
                from_value: None,
                from_string: Some("In Progress".to_string()),
                to_value: None,
                to_string: Some("Done".to_string()),
                changed_at: now,
            },
        ];
        history_repo.batch_insert(&history).unwrap();

        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&snapshot_repo),
        );

        let result = use_case.execute("PROJ", "100").unwrap();

        assert!(result.completed);
        assert_eq!(result.issues_processed, 1);
        // 3 snapshots: initial (Open) + In Progress + Done
        assert_eq!(result.snapshots_generated, 3);

        // Verify snapshot versions
        let snapshots = snapshot_repo.get_snapshots();
        assert_eq!(snapshots.len(), 3);

        // Check version ordering
        let mut versions: Vec<_> = snapshots.iter().map(|s| s.version).collect();
        versions.sort();
        assert_eq!(versions, vec![1, 2, 3]);
    }

    /// Test: Snapshot checkpoint infrastructure exists
    #[tokio::test]
    async fn test_snapshot_checkpoint_support() {
        let now = Utc::now();
        // Use zero-padded IDs for proper string ordering
        let issues: Vec<Issue> = (1..=10)
            .map(|i| create_test_issue(&format!("{:02}", i), &format!("PROJ-{}", i), "100", now))
            .collect();

        let issue_repo = Arc::new(MockIssueRepository::new());
        issue_repo.batch_insert(&issues).unwrap();

        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = GenerateSnapshotsUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&snapshot_repo),
        )
        .with_batch_size(3);

        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = Arc::clone(&progress_updates);

        let result = use_case
            .execute_with_progress("PROJ", "100", None, move |progress| {
                progress_clone.lock().unwrap().push(progress.clone());
            })
            .unwrap();

        assert!(result.completed);
        assert_eq!(result.issues_processed, 10);

        // Verify progress was reported
        let updates = progress_updates.lock().unwrap();
        assert!(!updates.is_empty());
    }

    /// Test: Raw data is stored alongside issues
    #[tokio::test]
    async fn test_raw_data_stored_with_issues() {
        let now = Utc::now();
        let issues = vec![create_test_issue("1", "PROJ-1", "100", now)];

        let jira_service = Arc::new(MockJiraService::new(vec![issues]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        let result = use_case
            .execute_resumable("PROJ", "100", None, |_| {})
            .await
            .unwrap();

        assert!(result.sync_result.success);

        // Verify raw_json is preserved in issues table
        let saved_issues = issue_repo.get_issues();
        assert_eq!(saved_issues.len(), 1);
        assert!(saved_issues[0].raw_json.is_some());
        assert!(
            saved_issues[0]
                .raw_json
                .as_ref()
                .unwrap()
                .contains("changelog")
        );
    }

    /// Test: Snapshot checkpoint is returned when snapshot generation fails
    #[tokio::test]
    async fn test_snapshot_checkpoint_returned_on_failure() {
        let now = Utc::now();
        let issues = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
        ];

        let jira_service = Arc::new(MockJiraService::new(vec![issues]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        let result = use_case
            .execute_resumable("PROJ", "100", None, |_| {})
            .await
            .unwrap();

        // Sync should succeed (issues and snapshots)
        assert!(result.sync_result.success);
        // No snapshot checkpoint on success
        assert!(result.snapshot_checkpoint.is_none());
    }

    /// Test: Resume from snapshot checkpoint skips issue sync
    #[tokio::test]
    async fn test_resume_from_snapshot_checkpoint() {
        use crate::infrastructure::config::SnapshotCheckpoint;

        let now = Utc::now();
        // Pre-populate issues (simulating they were already synced)
        let issues = vec![
            create_test_issue("1", "PROJ-1", "100", now - Duration::hours(2)),
            create_test_issue("2", "PROJ-2", "100", now - Duration::hours(1)),
        ];

        // Empty JIRA service (should not be called)
        let jira_service = Arc::new(MockJiraService::new(vec![]));
        let issue_repo = Arc::new(MockIssueRepository::new());
        // Pre-populate issues
        issue_repo.batch_insert(&issues).unwrap();

        let history_repo = Arc::new(MockChangeHistoryRepository::new());
        let metadata_repo = Arc::new(MockMetadataRepository::new());
        let sync_history_repo = Arc::new(MockSyncHistoryRepository::new());
        let snapshot_repo = Arc::new(MockIssueSnapshotRepository::new());

        let use_case = SyncProjectUseCase::new(
            Arc::clone(&issue_repo),
            Arc::clone(&history_repo),
            Arc::clone(&metadata_repo),
            Arc::clone(&sync_history_repo),
            Arc::clone(&snapshot_repo),
            Arc::clone(&jira_service),
        );

        // Resume from snapshot checkpoint (skipping issue sync)
        let snapshot_checkpoint = SnapshotCheckpoint {
            last_issue_id: "1".to_string(),
            last_issue_key: "PROJ-1".to_string(),
            issues_processed: 1,
            total_issues: 2,
            snapshots_generated: 1,
        };

        let result = use_case
            .execute_resumable_with_snapshot_checkpoint(
                "PROJ",
                "100",
                None, // No issue checkpoint
                Some(snapshot_checkpoint),
                |_| {},
            )
            .await
            .unwrap();

        assert!(result.sync_result.success);
        // Issue sync was skipped, JIRA service should not have been called for issues
        let fetch_calls = jira_service.get_fetch_calls();
        assert!(
            fetch_calls.is_empty(),
            "JIRA service should not be called when resuming from snapshot checkpoint"
        );
    }
}
