use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
