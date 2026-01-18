use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Represents a snapshot of an issue at a specific point in time.
/// Each snapshot captures the state of the issue after a change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSnapshot {
    pub issue_id: String,
    pub issue_key: String,
    pub project_id: String,
    /// Version number (1 = initial state, increments with each change)
    pub version: i32,
    /// When this snapshot became valid (the change timestamp)
    pub valid_from: DateTime<Utc>,
    /// When this snapshot was superseded (next change timestamp, None if current)
    pub valid_to: Option<DateTime<Utc>>,
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub issue_type: Option<String>,
    pub resolution: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub fix_versions: Option<Vec<String>>,
    pub sprint: Option<String>,
    pub parent_key: Option<String>,
    /// Raw JIRA API response (only available for current snapshot)
    pub raw_data: Option<JsonValue>,
    /// The date when the issue was last updated in JIRA (from the original issue)
    pub updated_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl IssueSnapshot {
    /// Create a new snapshot
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issue_id: String,
        issue_key: String,
        project_id: String,
        version: i32,
        valid_from: DateTime<Utc>,
        valid_to: Option<DateTime<Utc>>,
        summary: String,
        description: Option<String>,
        status: Option<String>,
        priority: Option<String>,
        assignee: Option<String>,
        reporter: Option<String>,
        issue_type: Option<String>,
        resolution: Option<String>,
        labels: Option<Vec<String>>,
        components: Option<Vec<String>>,
        fix_versions: Option<Vec<String>>,
        sprint: Option<String>,
        parent_key: Option<String>,
        raw_data: Option<JsonValue>,
        updated_date: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            issue_id,
            issue_key,
            project_id,
            version,
            valid_from,
            valid_to,
            summary,
            description,
            status,
            priority,
            assignee,
            reporter,
            issue_type,
            resolution,
            labels,
            components,
            fix_versions,
            sprint,
            parent_key,
            raw_data,
            updated_date,
            created_at: Utc::now(),
        }
    }

    /// Check if this snapshot is the current (latest) version
    pub fn is_current(&self) -> bool {
        self.valid_to.is_none()
    }
}
