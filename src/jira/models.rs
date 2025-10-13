use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
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
    pub created_date: Option<DateTime<Utc>>,
    pub updated_date: Option<DateTime<Utc>>,
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
            created_date,
            updated_date,
        }
    }
}
