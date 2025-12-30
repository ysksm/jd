use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
