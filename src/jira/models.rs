use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

// Metadata models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueType {
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub subtask: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub description: Option<String>,
    pub lead: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixVersion {
    pub name: String,
    pub description: Option<String>,
    pub released: bool,
    pub release_date: Option<DateTime<Utc>>,
}

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
    pub parent_key: Option<String>,
    pub created_date: Option<DateTime<Utc>>,
    pub updated_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<String>,
}

/// Represents a single change item in the changelog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeHistoryItem {
    pub issue_id: String,
    pub issue_key: String,
    pub history_id: String,
    pub author_account_id: Option<String>,
    pub author_display_name: Option<String>,
    pub field: String,
    pub field_type: Option<String>,
    pub from_value: Option<String>,
    pub from_string: Option<String>,
    pub to_value: Option<String>,
    pub to_string: Option<String>,
    pub changed_at: DateTime<Utc>,
}

impl ChangeHistoryItem {
    /// Extract change history items from the raw JSON of an issue
    pub fn extract_from_raw_json(issue_id: &str, issue_key: &str, raw_json: &str) -> Vec<Self> {
        let mut items = Vec::new();

        let json: Value = match serde_json::from_str(raw_json) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("Failed to parse raw_json for {}: {}", issue_key, e);
                return items;
            }
        };

        // Navigate to changelog.histories
        let changelog = json.get("changelog");
        if changelog.is_none() {
            log::debug!("No changelog found in raw_json for {} (this is normal for issues with no changes)", issue_key);
            return items;
        }

        let histories = match changelog.and_then(|c| c.get("histories")) {
            Some(Value::Array(arr)) => arr,
            _ => {
                log::debug!("No histories array found in changelog for {}", issue_key);
                return items;
            }
        };

        for history in histories {
            let history_id = history
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let author_account_id = history
                .get("author")
                .and_then(|a| a.get("accountId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let author_display_name = history
                .get("author")
                .and_then(|a| a.get("displayName"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let changed_at = history
                .get("created")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            // Get the items array from the history
            let change_items = match history.get("items") {
                Some(Value::Array(arr)) => arr,
                _ => continue,
            };

            for item in change_items {
                let field = match item.get("field").and_then(|v| v.as_str()) {
                    Some(f) => f.to_string(),
                    None => continue,
                };

                let field_type = item
                    .get("fieldtype")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let from_value = item
                    .get("from")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let from_string = item
                    .get("fromString")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let to_value = item
                    .get("to")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let to_string = item
                    .get("toString")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                items.push(ChangeHistoryItem {
                    issue_id: issue_id.to_string(),
                    issue_key: issue_key.to_string(),
                    history_id: history_id.clone(),
                    author_account_id: author_account_id.clone(),
                    author_display_name: author_display_name.clone(),
                    field,
                    field_type,
                    from_value,
                    from_string,
                    to_value,
                    to_string,
                    changed_at,
                });
            }
        }

        items
    }
}

impl From<jira_api::Project> for Project {
    fn from(p: jira_api::Project) -> Self {
        Self {
            id: p.id,
            key: p.key,
            name: p.name,
            description: p.description,
        }
    }
}

impl From<jira_api::Issue> for Issue {
    fn from(i: jira_api::Issue) -> Self {
        // Extract fields from the JSON Value
        let fields = &i.fields;

        let project_id = fields["project"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let summary = fields["summary"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let description = fields["description"]
            .as_str()
            .map(|s| s.to_string());

        let status = fields["status"]["name"]
            .as_str()
            .map(|s| s.to_string());

        let priority = fields["priority"]["name"]
            .as_str()
            .map(|s| s.to_string());

        let assignee = fields["assignee"]["displayName"]
            .as_str()
            .map(|s| s.to_string());

        let reporter = fields["reporter"]["displayName"]
            .as_str()
            .map(|s| s.to_string());

        // Extract metadata
        let issue_type = fields["issuetype"]["name"]
            .as_str()
            .map(|s| s.to_string());

        let resolution = fields["resolution"]["name"]
            .as_str()
            .map(|s| s.to_string());

        // Extract labels (array of strings)
        let labels = fields["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        // Extract components (array of objects with "name" field)
        let components = fields["components"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        // Extract fix versions (array of objects with "name" field)
        let fix_versions = fields["fixVersions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        // Extract parent key (for subtasks)
        let parent_key = fields["parent"]["key"]
            .as_str()
            .map(|s| s.to_string());

        let created_date = fields["created"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let updated_date = fields["updated"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Self {
            id: i.id,
            project_id,
            key: i.key,
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
            parent_key,
            created_date,
            updated_date,
            raw_json: None,
        }
    }
}
