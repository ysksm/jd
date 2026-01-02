use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use log::info;

use crate::domain::entities::{ChangeHistoryItem, Issue, IssueSnapshot};
use crate::domain::error::DomainResult;
use crate::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository,
};

/// Result of snapshot generation
#[derive(Debug, Clone)]
pub struct SnapshotGenerationResult {
    pub project_key: String,
    pub issues_processed: usize,
    pub snapshots_generated: usize,
}

impl SnapshotGenerationResult {
    pub fn new(project_key: String, issues_processed: usize, snapshots_generated: usize) -> Self {
        Self {
            project_key,
            issues_processed,
            snapshots_generated,
        }
    }
}

pub struct GenerateSnapshotsUseCase<I, C, S>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
    S: IssueSnapshotRepository,
{
    issue_repository: Arc<I>,
    change_history_repository: Arc<C>,
    snapshot_repository: Arc<S>,
}

impl<I, C, S> GenerateSnapshotsUseCase<I, C, S>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
    S: IssueSnapshotRepository,
{
    pub fn new(
        issue_repository: Arc<I>,
        change_history_repository: Arc<C>,
        snapshot_repository: Arc<S>,
    ) -> Self {
        Self {
            issue_repository,
            change_history_repository,
            snapshot_repository,
        }
    }

    /// Generate snapshots for all issues in a project
    pub fn execute(
        &self,
        project_key: &str,
        project_id: &str,
    ) -> DomainResult<SnapshotGenerationResult> {
        info!("Generating snapshots for project: {}", project_key);

        let issues = self.issue_repository.find_by_project(project_id)?;
        let mut total_snapshots = 0;

        for issue in &issues {
            let snapshots = self.generate_snapshots_for_issue(issue)?;
            if !snapshots.is_empty() {
                // Delete existing snapshots and insert new ones
                self.snapshot_repository.delete_by_issue_id(&issue.id)?;
                self.snapshot_repository.batch_insert(&snapshots)?;
                total_snapshots += snapshots.len();
            }
        }

        info!(
            "Generated {} snapshots for {} issues in project {}",
            total_snapshots,
            issues.len(),
            project_key
        );

        Ok(SnapshotGenerationResult::new(
            project_key.to_string(),
            issues.len(),
            total_snapshots,
        ))
    }

    /// Generate snapshots for a single issue
    fn generate_snapshots_for_issue(&self, issue: &Issue) -> DomainResult<Vec<IssueSnapshot>> {
        let history = self
            .change_history_repository
            .find_by_issue_key(&issue.key)?;

        // Group changes by timestamp (same history entry may have multiple field changes)
        let grouped_changes = self.group_changes_by_timestamp(&history);

        // Sort timestamps chronologically (oldest first)
        let mut timestamps: Vec<_> = grouped_changes.keys().cloned().collect();
        timestamps.sort();

        let mut snapshots = Vec::new();

        // If no history, create a single snapshot from current state
        if timestamps.is_empty() {
            let created_at = issue.created_date.unwrap_or_else(Utc::now);
            let snapshot = self.create_snapshot_from_issue(issue, 1, created_at, None);
            snapshots.push(snapshot);
            return Ok(snapshots);
        }

        // Build snapshots by applying changes forward from initial state
        let mut current_state = self.build_initial_state(issue, &grouped_changes, &timestamps);
        let issue_created = issue.created_date.unwrap_or_else(Utc::now);

        // Version 1: Initial state (from creation to first change)
        let first_change_time = timestamps[0];
        let snapshot = IssueSnapshot::new(
            issue.id.clone(),
            issue.key.clone(),
            issue.project_id.clone(),
            1,
            issue_created,
            Some(first_change_time),
            current_state
                .get("summary")
                .cloned()
                .unwrap_or_else(|| issue.summary.clone()),
            current_state
                .get("description")
                .cloned()
                .or_else(|| issue.description.clone()),
            current_state
                .get("status")
                .cloned()
                .or_else(|| issue.status.clone()),
            current_state
                .get("priority")
                .cloned()
                .or_else(|| issue.priority.clone()),
            current_state
                .get("assignee")
                .cloned()
                .or_else(|| issue.assignee.clone()),
            current_state
                .get("reporter")
                .cloned()
                .or_else(|| issue.reporter.clone()),
            current_state
                .get("issuetype")
                .cloned()
                .or_else(|| issue.issue_type.clone()),
            current_state
                .get("resolution")
                .cloned()
                .or_else(|| issue.resolution.clone()),
            issue.labels.clone(),
            issue.components.clone(),
            issue.fix_versions.clone(),
            current_state
                .get("sprint")
                .cloned()
                .or_else(|| issue.sprint.clone()),
            current_state
                .get("parent")
                .cloned()
                .or_else(|| issue.parent_key.clone()),
        );
        snapshots.push(snapshot);

        // Apply each change to create subsequent snapshots
        for (i, &change_time) in timestamps.iter().enumerate() {
            let Some(changes) = grouped_changes.get(&change_time) else {
                // This should never happen, but skip gracefully if it does
                continue;
            };

            // Apply changes to current state
            for change in changes {
                let field_name = change.field.to_lowercase();
                if let Some(to_value) = &change.to_string {
                    current_state.insert(field_name, to_value.clone());
                } else if change.to_value.is_some() {
                    if let Some(to_val) = &change.to_value {
                        current_state.insert(field_name, to_val.clone());
                    }
                }
            }

            // Determine valid_to (next change time or None if this is the last)
            let valid_to = if i + 1 < timestamps.len() {
                Some(timestamps[i + 1])
            } else {
                None
            };

            let version = (i + 2) as i32; // Version 1 was initial, so changes start at version 2
            let snapshot = IssueSnapshot::new(
                issue.id.clone(),
                issue.key.clone(),
                issue.project_id.clone(),
                version,
                change_time,
                valid_to,
                current_state
                    .get("summary")
                    .cloned()
                    .unwrap_or_else(|| issue.summary.clone()),
                current_state
                    .get("description")
                    .cloned()
                    .or_else(|| issue.description.clone()),
                current_state
                    .get("status")
                    .cloned()
                    .or_else(|| issue.status.clone()),
                current_state
                    .get("priority")
                    .cloned()
                    .or_else(|| issue.priority.clone()),
                current_state
                    .get("assignee")
                    .cloned()
                    .or_else(|| issue.assignee.clone()),
                current_state
                    .get("reporter")
                    .cloned()
                    .or_else(|| issue.reporter.clone()),
                current_state
                    .get("issuetype")
                    .cloned()
                    .or_else(|| issue.issue_type.clone()),
                current_state
                    .get("resolution")
                    .cloned()
                    .or_else(|| issue.resolution.clone()),
                issue.labels.clone(),
                issue.components.clone(),
                issue.fix_versions.clone(),
                current_state
                    .get("sprint")
                    .cloned()
                    .or_else(|| issue.sprint.clone()),
                current_state
                    .get("parent")
                    .cloned()
                    .or_else(|| issue.parent_key.clone()),
            );
            snapshots.push(snapshot);
        }

        Ok(snapshots)
    }

    /// Group change history items by their timestamp
    fn group_changes_by_timestamp<'a>(
        &self,
        history: &'a [ChangeHistoryItem],
    ) -> HashMap<DateTime<Utc>, Vec<&'a ChangeHistoryItem>> {
        let mut grouped: HashMap<DateTime<Utc>, Vec<&'a ChangeHistoryItem>> = HashMap::new();
        for item in history {
            grouped.entry(item.changed_at).or_default().push(item);
        }
        grouped
    }

    /// Build the initial state by reversing all changes from the current state
    fn build_initial_state(
        &self,
        issue: &Issue,
        grouped_changes: &HashMap<DateTime<Utc>, Vec<&ChangeHistoryItem>>,
        timestamps: &[DateTime<Utc>],
    ) -> HashMap<String, String> {
        let mut state: HashMap<String, String> = HashMap::new();

        // Start with current values
        if let Some(v) = &issue.status {
            state.insert("status".to_string(), v.clone());
        }
        if let Some(v) = &issue.priority {
            state.insert("priority".to_string(), v.clone());
        }
        if let Some(v) = &issue.assignee {
            state.insert("assignee".to_string(), v.clone());
        }
        if let Some(v) = &issue.reporter {
            state.insert("reporter".to_string(), v.clone());
        }
        if let Some(v) = &issue.issue_type {
            state.insert("issuetype".to_string(), v.clone());
        }
        if let Some(v) = &issue.resolution {
            state.insert("resolution".to_string(), v.clone());
        }
        if let Some(v) = &issue.sprint {
            state.insert("sprint".to_string(), v.clone());
        }
        if let Some(v) = &issue.parent_key {
            state.insert("parent".to_string(), v.clone());
        }
        state.insert("summary".to_string(), issue.summary.clone());
        if let Some(v) = &issue.description {
            state.insert("description".to_string(), v.clone());
        }

        // Reverse changes from newest to oldest to get initial state
        for timestamp in timestamps.iter().rev() {
            if let Some(changes) = grouped_changes.get(timestamp) {
                for change in changes {
                    let field_name = change.field.to_lowercase();
                    // Revert to the "from" value
                    if let Some(from_value) = &change.from_string {
                        state.insert(field_name, from_value.clone());
                    } else if let Some(from_val) = &change.from_value {
                        state.insert(field_name, from_val.clone());
                    } else {
                        // If from_value is None, the field was initially empty
                        state.remove(&field_name);
                    }
                }
            }
        }

        state
    }

    /// Create a snapshot directly from the current issue state
    fn create_snapshot_from_issue(
        &self,
        issue: &Issue,
        version: i32,
        valid_from: DateTime<Utc>,
        valid_to: Option<DateTime<Utc>>,
    ) -> IssueSnapshot {
        IssueSnapshot::new(
            issue.id.clone(),
            issue.key.clone(),
            issue.project_id.clone(),
            version,
            valid_from,
            valid_to,
            issue.summary.clone(),
            issue.description.clone(),
            issue.status.clone(),
            issue.priority.clone(),
            issue.assignee.clone(),
            issue.reporter.clone(),
            issue.issue_type.clone(),
            issue.resolution.clone(),
            issue.labels.clone(),
            issue.components.clone(),
            issue.fix_versions.clone(),
            issue.sprint.clone(),
            issue.parent_key.clone(),
        )
    }
}
