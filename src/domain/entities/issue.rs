use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub project_id: String,
    pub key: String,
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
    pub created_date: Option<DateTime<Utc>>,
    pub updated_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<String>,
}

impl Issue {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project_id: String,
        key: String,
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
        created_date: Option<DateTime<Utc>>,
        updated_date: Option<DateTime<Utc>>,
        raw_json: Option<String>,
    ) -> Self {
        Self {
            id,
            project_id,
            key,
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
            created_date,
            updated_date,
            raw_json,
        }
    }
}
