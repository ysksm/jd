use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde_json::{Value as JsonValue, json};

use crate::domain::entities::{ChangeHistoryItem, Issue, IssueSnapshot};
use crate::domain::error::DomainResult;
use crate::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository,
};
use crate::infrastructure::config::SnapshotCheckpoint;

/// Mapping from changelog field names to raw_data field paths and their structure type
#[derive(Debug, Clone, Copy)]
enum FieldType {
    /// Direct string value (e.g., summary, description)
    DirectString,
    /// Object with "name" property (e.g., status, priority, issuetype, resolution)
    ObjectWithName,
    /// Object with "displayName" property (e.g., assignee, reporter)
    ObjectWithDisplayName,
    /// Object with "value" property (e.g., some custom fields)
    ObjectWithValue,
    /// Array of strings (e.g., labels)
    ArrayOfStrings,
    /// Array of objects with "name" property (e.g., components, fixVersions)
    ArrayOfObjectsWithName,
    /// Unknown structure - try multiple approaches
    Unknown,
}

/// Get the field type for a given changelog field name
fn get_field_type(field_name: &str) -> FieldType {
    match field_name.to_lowercase().as_str() {
        "summary" | "description" | "environment" => FieldType::DirectString,
        "status" | "priority" | "issuetype" | "resolution" | "security" => {
            FieldType::ObjectWithName
        }
        "assignee" | "reporter" | "creator" => FieldType::ObjectWithDisplayName,
        "labels" => FieldType::ArrayOfStrings,
        "components" | "fixversions" | "versions" | "affectedversions" => {
            FieldType::ArrayOfObjectsWithName
        }
        "sprint" => FieldType::ObjectWithName, // Sprint is typically object with name
        "parent" => FieldType::ObjectWithName, // Parent issue
        _ if field_name.starts_with("customfield_") => FieldType::Unknown,
        _ => FieldType::Unknown,
    }
}

/// Get the raw_data field path for a changelog field name
fn get_raw_data_field_name(changelog_field: &str) -> &str {
    match changelog_field.to_lowercase().as_str() {
        "issuetype" => "issuetype",
        "fixversions" => "fixVersions",
        "affectedversions" => "versions",
        _ => changelog_field,
    }
}

/// Apply a change in reverse to raw_data (revert to previous state)
fn apply_change_reverse_to_raw_data(raw_data: &mut JsonValue, change: &ChangeHistoryItem) {
    let field_name = get_raw_data_field_name(&change.field);
    let field_type = get_field_type(&change.field);

    let fields = match raw_data.get_mut("fields") {
        Some(f) => f,
        None => return,
    };

    // Get the "from" value to restore
    let from_string = change.from_string.as_deref();
    let from_value = change.from_value.as_deref();

    match field_type {
        FieldType::DirectString => {
            // Direct string replacement
            if let Some(from_str) = from_string.or(from_value) {
                fields[field_name] = json!(from_str);
            } else {
                // Field was empty before
                fields[field_name] = JsonValue::Null;
            }
        }
        FieldType::ObjectWithName => {
            if let Some(from_str) = from_string {
                // Preserve existing object structure if possible, just update name
                if let Some(obj) = fields.get_mut(field_name) {
                    if obj.is_object() {
                        obj["name"] = json!(from_str);
                        if let Some(id) = from_value {
                            obj["id"] = json!(id);
                        }
                    } else {
                        fields[field_name] = json!({"name": from_str, "id": from_value});
                    }
                } else {
                    fields[field_name] = json!({"name": from_str, "id": from_value});
                }
            } else {
                // Field was null before
                fields[field_name] = JsonValue::Null;
            }
        }
        FieldType::ObjectWithDisplayName => {
            if let Some(from_str) = from_string {
                if let Some(obj) = fields.get_mut(field_name) {
                    if obj.is_object() {
                        obj["displayName"] = json!(from_str);
                        if let Some(id) = from_value {
                            obj["accountId"] = json!(id);
                        }
                    } else {
                        fields[field_name] =
                            json!({"displayName": from_str, "accountId": from_value});
                    }
                } else {
                    fields[field_name] = json!({"displayName": from_str, "accountId": from_value});
                }
            } else {
                fields[field_name] = JsonValue::Null;
            }
        }
        FieldType::ObjectWithValue => {
            if let Some(from_str) = from_string.or(from_value) {
                if let Some(obj) = fields.get_mut(field_name) {
                    if obj.is_object() {
                        obj["value"] = json!(from_str);
                    } else {
                        fields[field_name] = json!({"value": from_str});
                    }
                } else {
                    fields[field_name] = json!({"value": from_str});
                }
            } else {
                fields[field_name] = JsonValue::Null;
            }
        }
        FieldType::ArrayOfStrings => {
            // For labels, parse the comma-separated from_string
            if let Some(from_str) = from_string {
                if from_str.is_empty() {
                    fields[field_name] = json!([]);
                } else {
                    let items: Vec<&str> = from_str.split_whitespace().collect();
                    fields[field_name] = json!(items);
                }
            } else {
                fields[field_name] = json!([]);
            }
        }
        FieldType::ArrayOfObjectsWithName => {
            // For components/versions, the from_string might be comma-separated names
            if let Some(from_str) = from_string {
                if from_str.is_empty() {
                    fields[field_name] = json!([]);
                } else {
                    // Split by comma and create objects
                    let items: Vec<JsonValue> = from_str
                        .split(',')
                        .map(|s| json!({"name": s.trim()}))
                        .collect();
                    fields[field_name] = json!(items);
                }
            } else {
                fields[field_name] = json!([]);
            }
        }
        FieldType::Unknown => {
            // Try to intelligently handle unknown fields
            // First check if there's an existing value to understand the structure
            if let Some(existing) = fields.get(field_name) {
                if existing.is_object() {
                    // Try to determine what property to update
                    if existing.get("name").is_some() {
                        if let Some(from_str) = from_string {
                            let mut obj = existing.clone();
                            obj["name"] = json!(from_str);
                            if let Some(id) = from_value {
                                obj["id"] = json!(id);
                            }
                            fields[field_name] = obj;
                        } else {
                            fields[field_name] = JsonValue::Null;
                        }
                    } else if existing.get("value").is_some() {
                        if let Some(from_str) = from_string.or(from_value) {
                            let mut obj = existing.clone();
                            obj["value"] = json!(from_str);
                            fields[field_name] = obj;
                        } else {
                            fields[field_name] = JsonValue::Null;
                        }
                    } else if existing.get("displayName").is_some() {
                        if let Some(from_str) = from_string {
                            let mut obj = existing.clone();
                            obj["displayName"] = json!(from_str);
                            fields[field_name] = obj;
                        } else {
                            fields[field_name] = JsonValue::Null;
                        }
                    } else {
                        // Unknown object structure, try name first
                        if let Some(from_str) = from_string.or(from_value) {
                            fields[field_name] = json!({"name": from_str});
                        } else {
                            fields[field_name] = JsonValue::Null;
                        }
                    }
                } else if existing.is_array() {
                    // Array field - try to reconstruct
                    if let Some(from_str) = from_string {
                        if from_str.is_empty() {
                            fields[field_name] = json!([]);
                        } else {
                            let items: Vec<JsonValue> = from_str
                                .split(',')
                                .map(|s| json!({"name": s.trim()}))
                                .collect();
                            fields[field_name] = json!(items);
                        }
                    } else {
                        fields[field_name] = json!([]);
                    }
                } else {
                    // Simple value
                    if let Some(from_str) = from_string.or(from_value) {
                        fields[field_name] = json!(from_str);
                    } else {
                        fields[field_name] = JsonValue::Null;
                    }
                }
            } else {
                // No existing value, create based on from_string/from_value
                if let Some(from_str) = from_string.or(from_value) {
                    fields[field_name] = json!(from_str);
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
        }
    }
}

/// Apply multiple changes in reverse order to raw_data
fn apply_changes_reverse_to_raw_data(raw_data: &mut JsonValue, changes: &[&ChangeHistoryItem]) {
    for change in changes {
        apply_change_reverse_to_raw_data(raw_data, change);
    }
}

/// Default batch size for processing issues
const DEFAULT_BATCH_SIZE: usize = 500;

/// Result of snapshot generation
#[derive(Debug, Clone)]
pub struct SnapshotGenerationResult {
    pub project_key: String,
    pub issues_processed: usize,
    pub snapshots_generated: usize,
    /// Checkpoint for resuming (None if completed successfully)
    pub checkpoint: Option<SnapshotCheckpoint>,
    /// Whether the generation completed fully
    pub completed: bool,
}

impl SnapshotGenerationResult {
    pub fn new(project_key: String, issues_processed: usize, snapshots_generated: usize) -> Self {
        Self {
            project_key,
            issues_processed,
            snapshots_generated,
            checkpoint: None,
            completed: true,
        }
    }

    pub fn with_checkpoint(mut self, checkpoint: SnapshotCheckpoint) -> Self {
        self.checkpoint = Some(checkpoint);
        self.completed = false;
        self
    }
}

/// Progress information for snapshot generation
#[derive(Debug, Clone)]
pub struct SnapshotProgress {
    pub issues_processed: usize,
    pub total_issues: usize,
    pub snapshots_generated: usize,
    pub current_issue_key: String,
    /// ID of the last successfully processed issue (for checkpoint)
    pub last_issue_id: String,
    /// Key of the last successfully processed issue (for checkpoint)
    pub last_issue_key: String,
}

/// Create a snapshot checkpoint from progress data
pub fn create_snapshot_checkpoint(
    last_issue_id: &str,
    last_issue_key: &str,
    issues_processed: usize,
    total_issues: usize,
    snapshots_generated: usize,
) -> SnapshotCheckpoint {
    SnapshotCheckpoint {
        last_issue_id: last_issue_id.to_string(),
        last_issue_key: last_issue_key.to_string(),
        issues_processed,
        total_issues,
        snapshots_generated,
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
    batch_size: usize,
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
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Set custom batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Generate snapshots for all issues in a project (simple API, no resume)
    pub fn execute(
        &self,
        project_key: &str,
        project_id: &str,
    ) -> DomainResult<SnapshotGenerationResult> {
        self.execute_with_progress(project_key, project_id, None, |_| {})
    }

    /// Generate snapshots with checkpoint support and progress callback
    ///
    /// # Arguments
    /// * `project_key` - The JIRA project key
    /// * `project_id` - The JIRA project ID
    /// * `checkpoint` - Optional checkpoint to resume from
    /// * `on_progress` - Callback called after each batch with progress info
    pub fn execute_with_progress<F>(
        &self,
        project_key: &str,
        project_id: &str,
        checkpoint: Option<SnapshotCheckpoint>,
        mut on_progress: F,
    ) -> DomainResult<SnapshotGenerationResult>
    where
        F: FnMut(&SnapshotProgress),
    {
        info!("Generating snapshots for project: {}", project_key);

        // Get total count for progress reporting
        let total_issues = self.issue_repository.count_by_project(project_id)?;

        if total_issues == 0 {
            info!("No issues found for project {}", project_key);
            return Ok(SnapshotGenerationResult::new(project_key.to_string(), 0, 0));
        }

        // Determine starting point
        let (start_after_id, mut issues_processed, mut total_snapshots) =
            if let Some(ref cp) = checkpoint {
                info!(
                    "Resuming from checkpoint: {} issues processed, {} snapshots generated",
                    cp.issues_processed, cp.snapshots_generated
                );
                (
                    Some(cp.last_issue_id.clone()),
                    cp.issues_processed,
                    cp.snapshots_generated,
                )
            } else {
                (None, 0, 0)
            };

        // Begin transaction for the entire operation
        self.snapshot_repository.begin_transaction()?;

        let result = self.process_batches(
            project_key,
            project_id,
            total_issues,
            start_after_id,
            &mut issues_processed,
            &mut total_snapshots,
            &mut on_progress,
        );

        match result {
            Ok(()) => {
                // Commit on success
                self.snapshot_repository.commit_transaction()?;
                info!(
                    "Generated {} snapshots for {} issues in project {}",
                    total_snapshots, issues_processed, project_key
                );
                Ok(SnapshotGenerationResult::new(
                    project_key.to_string(),
                    issues_processed,
                    total_snapshots,
                ))
            }
            Err(e) => {
                // Rollback and return checkpoint for resume
                if let Err(rollback_err) = self.snapshot_repository.rollback_transaction() {
                    warn!("Failed to rollback transaction: {}", rollback_err);
                }
                Err(e)
            }
        }
    }

    /// Process issues in batches
    fn process_batches<F>(
        &self,
        _project_key: &str,
        project_id: &str,
        total_issues: usize,
        start_after_id: Option<String>,
        issues_processed: &mut usize,
        total_snapshots: &mut usize,
        on_progress: &mut F,
    ) -> DomainResult<()>
    where
        F: FnMut(&SnapshotProgress),
    {
        let mut last_issue_id = start_after_id;
        let mut batch_num = 0;

        loop {
            batch_num += 1;

            // Fetch a batch of issues
            let page = if let Some(ref after_id) = last_issue_id {
                self.issue_repository.find_by_project_after_id(
                    project_id,
                    after_id,
                    self.batch_size,
                )?
            } else {
                self.issue_repository
                    .find_by_project_paginated(project_id, 0, self.batch_size)?
            };

            if page.issues.is_empty() {
                debug!("No more issues to process");
                break;
            }

            debug!(
                "Processing batch {}: {} issues (total processed: {})",
                batch_num,
                page.issues.len(),
                *issues_processed
            );

            // Collect all snapshots for this batch
            let mut batch_snapshots = Vec::new();
            let mut current_issue_key = String::new();

            for issue in &page.issues {
                current_issue_key = issue.key.clone();

                // Delete existing snapshots for this issue
                self.snapshot_repository.delete_by_issue_id(&issue.id)?;

                // Generate snapshots for this issue
                let snapshots = self.generate_snapshots_for_issue(issue)?;
                batch_snapshots.extend(snapshots);
            }

            // Bulk insert all snapshots for this batch
            let batch_snapshot_count = batch_snapshots.len();
            if !batch_snapshots.is_empty() {
                self.snapshot_repository.bulk_insert(&batch_snapshots)?;
            }

            // Update counters
            *issues_processed += page.issues.len();
            *total_snapshots += batch_snapshot_count;

            // Update last_issue_id for next batch
            if let Some(last_issue) = page.issues.last() {
                last_issue_id = Some(last_issue.id.clone());
            }

            // Report progress with last issue info for checkpoint
            let last_id = last_issue_id.clone().unwrap_or_default();
            let last_key = page
                .issues
                .last()
                .map(|i| i.key.clone())
                .unwrap_or_default();
            let progress = SnapshotProgress {
                issues_processed: *issues_processed,
                total_issues,
                snapshots_generated: *total_snapshots,
                current_issue_key,
                last_issue_id: last_id,
                last_issue_key: last_key,
            };
            on_progress(&progress);

            debug!(
                "Batch {} complete: {} snapshots generated (total: {})",
                batch_num, batch_snapshot_count, *total_snapshots
            );

            if !page.has_more {
                break;
            }
        }

        Ok(())
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

        // Parse raw_json once for the current snapshot
        let current_raw_data = Self::parse_raw_json(&issue.raw_json);

        // If no history, create a single snapshot from current state
        if timestamps.is_empty() {
            let created_at = issue.created_date.unwrap_or_else(Utc::now);
            // This is the only (and current) snapshot, so include raw_data
            let snapshot =
                self.create_snapshot_from_issue(issue, 1, created_at, None, current_raw_data);
            snapshots.push(snapshot);
            return Ok(snapshots);
        }

        // Build snapshots by applying changes forward from initial state
        let mut current_state = self.build_initial_state(issue, &grouped_changes, &timestamps);
        let issue_created = issue.created_date.unwrap_or_else(Utc::now);

        // Build initial raw_data by applying ALL changes in reverse to current raw_data
        let mut snapshot_raw_data = current_raw_data.clone();
        if let Some(ref mut raw_data) = snapshot_raw_data {
            // Apply all changes in reverse order (newest to oldest)
            for timestamp in timestamps.iter().rev() {
                if let Some(changes) = grouped_changes.get(timestamp) {
                    apply_changes_reverse_to_raw_data(raw_data, changes);
                }
            }
        }

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
            Self::extract_labels_from_raw_data(&snapshot_raw_data).or_else(|| issue.labels.clone()),
            Self::extract_components_from_raw_data(&snapshot_raw_data)
                .or_else(|| issue.components.clone()),
            Self::extract_fix_versions_from_raw_data(&snapshot_raw_data)
                .or_else(|| issue.fix_versions.clone()),
            current_state
                .get("sprint")
                .cloned()
                .or_else(|| issue.sprint.clone()),
            current_state
                .get("parent")
                .cloned()
                .or_else(|| issue.parent_key.clone()),
            snapshot_raw_data.clone(),
            issue.updated_date,
        );
        snapshots.push(snapshot);

        // Apply each change to create subsequent snapshots
        for (i, &change_time) in timestamps.iter().enumerate() {
            let Some(changes) = grouped_changes.get(&change_time) else {
                // This should never happen, but skip gracefully if it does
                continue;
            };

            // Apply changes to current state (for basic fields)
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

            // Apply changes forward to raw_data
            if let Some(ref mut raw_data) = snapshot_raw_data {
                Self::apply_changes_forward_to_raw_data(raw_data, changes);
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
                Self::extract_labels_from_raw_data(&snapshot_raw_data)
                    .or_else(|| issue.labels.clone()),
                Self::extract_components_from_raw_data(&snapshot_raw_data)
                    .or_else(|| issue.components.clone()),
                Self::extract_fix_versions_from_raw_data(&snapshot_raw_data)
                    .or_else(|| issue.fix_versions.clone()),
                current_state
                    .get("sprint")
                    .cloned()
                    .or_else(|| issue.sprint.clone()),
                current_state
                    .get("parent")
                    .cloned()
                    .or_else(|| issue.parent_key.clone()),
                snapshot_raw_data.clone(),
                issue.updated_date,
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
        raw_data: Option<JsonValue>,
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
            raw_data,
            issue.updated_date,
        )
    }

    /// Parse raw_json string to JsonValue
    fn parse_raw_json(raw_json: &Option<String>) -> Option<JsonValue> {
        raw_json.as_ref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Apply changes forward to raw_data (opposite of reverse - use to_value/to_string)
    fn apply_changes_forward_to_raw_data(raw_data: &mut JsonValue, changes: &[&ChangeHistoryItem]) {
        for change in changes {
            Self::apply_change_forward_to_raw_data(raw_data, change);
        }
    }

    /// Apply a single change forward to raw_data
    fn apply_change_forward_to_raw_data(raw_data: &mut JsonValue, change: &ChangeHistoryItem) {
        let field_name = get_raw_data_field_name(&change.field);
        let field_type = get_field_type(&change.field);

        let fields = match raw_data.get_mut("fields") {
            Some(f) => f,
            None => return,
        };

        // Get the "to" value to apply
        let to_string = change.to_string.as_deref();
        let to_value = change.to_value.as_deref();

        match field_type {
            FieldType::DirectString => {
                if let Some(to_str) = to_string.or(to_value) {
                    fields[field_name] = json!(to_str);
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
            FieldType::ObjectWithName => {
                if let Some(to_str) = to_string {
                    if let Some(obj) = fields.get_mut(field_name) {
                        if obj.is_object() {
                            obj["name"] = json!(to_str);
                            if let Some(id) = to_value {
                                obj["id"] = json!(id);
                            }
                        } else {
                            fields[field_name] = json!({"name": to_str, "id": to_value});
                        }
                    } else {
                        fields[field_name] = json!({"name": to_str, "id": to_value});
                    }
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
            FieldType::ObjectWithDisplayName => {
                if let Some(to_str) = to_string {
                    if let Some(obj) = fields.get_mut(field_name) {
                        if obj.is_object() {
                            obj["displayName"] = json!(to_str);
                            if let Some(id) = to_value {
                                obj["accountId"] = json!(id);
                            }
                        } else {
                            fields[field_name] =
                                json!({"displayName": to_str, "accountId": to_value});
                        }
                    } else {
                        fields[field_name] = json!({"displayName": to_str, "accountId": to_value});
                    }
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
            FieldType::ObjectWithValue => {
                if let Some(to_str) = to_string.or(to_value) {
                    if let Some(obj) = fields.get_mut(field_name) {
                        if obj.is_object() {
                            obj["value"] = json!(to_str);
                        } else {
                            fields[field_name] = json!({"value": to_str});
                        }
                    } else {
                        fields[field_name] = json!({"value": to_str});
                    }
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
            FieldType::ArrayOfStrings => {
                if let Some(to_str) = to_string {
                    if to_str.is_empty() {
                        fields[field_name] = json!([]);
                    } else {
                        let items: Vec<&str> = to_str.split_whitespace().collect();
                        fields[field_name] = json!(items);
                    }
                } else {
                    fields[field_name] = json!([]);
                }
            }
            FieldType::ArrayOfObjectsWithName => {
                if let Some(to_str) = to_string {
                    if to_str.is_empty() {
                        fields[field_name] = json!([]);
                    } else {
                        let items: Vec<JsonValue> = to_str
                            .split(',')
                            .map(|s| json!({"name": s.trim()}))
                            .collect();
                        fields[field_name] = json!(items);
                    }
                } else {
                    fields[field_name] = json!([]);
                }
            }
            FieldType::Unknown => {
                // Try to intelligently handle unknown fields
                if let Some(existing) = fields.get(field_name).cloned() {
                    if existing.is_object() {
                        if existing.get("name").is_some() {
                            if let Some(to_str) = to_string {
                                let mut obj = existing;
                                obj["name"] = json!(to_str);
                                if let Some(id) = to_value {
                                    obj["id"] = json!(id);
                                }
                                fields[field_name] = obj;
                            } else {
                                fields[field_name] = JsonValue::Null;
                            }
                        } else if existing.get("value").is_some() {
                            if let Some(to_str) = to_string.or(to_value) {
                                let mut obj = existing;
                                obj["value"] = json!(to_str);
                                fields[field_name] = obj;
                            } else {
                                fields[field_name] = JsonValue::Null;
                            }
                        } else if existing.get("displayName").is_some() {
                            if let Some(to_str) = to_string {
                                let mut obj = existing;
                                obj["displayName"] = json!(to_str);
                                fields[field_name] = obj;
                            } else {
                                fields[field_name] = JsonValue::Null;
                            }
                        } else if let Some(to_str) = to_string.or(to_value) {
                            fields[field_name] = json!({"name": to_str});
                        } else {
                            fields[field_name] = JsonValue::Null;
                        }
                    } else if existing.is_array() {
                        if let Some(to_str) = to_string {
                            if to_str.is_empty() {
                                fields[field_name] = json!([]);
                            } else {
                                let items: Vec<JsonValue> = to_str
                                    .split(',')
                                    .map(|s| json!({"name": s.trim()}))
                                    .collect();
                                fields[field_name] = json!(items);
                            }
                        } else {
                            fields[field_name] = json!([]);
                        }
                    } else if let Some(to_str) = to_string.or(to_value) {
                        fields[field_name] = json!(to_str);
                    } else {
                        fields[field_name] = JsonValue::Null;
                    }
                } else if let Some(to_str) = to_string.or(to_value) {
                    fields[field_name] = json!(to_str);
                } else {
                    fields[field_name] = JsonValue::Null;
                }
            }
        }
    }

    /// Extract labels from raw_data as Vec<String>
    fn extract_labels_from_raw_data(raw_data: &Option<JsonValue>) -> Option<Vec<String>> {
        raw_data.as_ref().and_then(|data| {
            data.get("fields")
                .and_then(|f| f.get("labels"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect()
                })
        })
    }

    /// Extract components from raw_data as Vec<String>
    fn extract_components_from_raw_data(raw_data: &Option<JsonValue>) -> Option<Vec<String>> {
        raw_data.as_ref().and_then(|data| {
            data.get("fields")
                .and_then(|f| f.get("components"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            item.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
        })
    }

    /// Extract fix_versions from raw_data as Vec<String>
    fn extract_fix_versions_from_raw_data(raw_data: &Option<JsonValue>) -> Option<Vec<String>> {
        raw_data.as_ref().and_then(|data| {
            data.get("fields")
                .and_then(|f| f.get("fixVersions"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            item.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
        })
    }
}
