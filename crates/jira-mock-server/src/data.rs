//! Data models and storage for the mock JIRA server

#[allow(unused_imports)]
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Main data store for the mock server
#[derive(Debug, Default)]
pub struct DataStore {
    pub projects: RwLock<Vec<Project>>,
    pub issues: RwLock<HashMap<String, Issue>>,
    pub statuses: RwLock<HashMap<String, Vec<Status>>>,
    pub priorities: RwLock<Vec<Priority>>,
    pub issue_types: RwLock<HashMap<String, Vec<IssueType>>>,
    pub components: RwLock<HashMap<String, Vec<Component>>>,
    pub versions: RwLock<HashMap<String, Vec<Version>>>,
    pub fields: RwLock<Vec<Field>>,
    pub issue_links: RwLock<Vec<IssueLink>>,
    pub transitions: RwLock<HashMap<String, Vec<Transition>>>,
    data_dir: PathBuf,
}

impl DataStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            ..Default::default()
        }
    }

    /// Load all data from JSON files
    pub fn load(&self) -> Result<(), String> {
        if !self.data_dir.exists() {
            fs::create_dir_all(&self.data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
            self.save_defaults()?;
            return Ok(());
        }

        // Load projects
        if let Ok(data) = fs::read_to_string(self.data_dir.join("projects.json")) {
            if let Ok(projects) = serde_json::from_str(&data) {
                *self.projects.write() = projects;
            }
        }

        // Load issues
        if let Ok(data) = fs::read_to_string(self.data_dir.join("issues.json")) {
            if let Ok(issues) = serde_json::from_str(&data) {
                *self.issues.write() = issues;
            }
        }

        // Load statuses
        if let Ok(data) = fs::read_to_string(self.data_dir.join("statuses.json")) {
            if let Ok(statuses) = serde_json::from_str(&data) {
                *self.statuses.write() = statuses;
            }
        }

        // Load priorities
        if let Ok(data) = fs::read_to_string(self.data_dir.join("priorities.json")) {
            if let Ok(priorities) = serde_json::from_str(&data) {
                *self.priorities.write() = priorities;
            }
        }

        // Load issue types
        if let Ok(data) = fs::read_to_string(self.data_dir.join("issue_types.json")) {
            if let Ok(issue_types) = serde_json::from_str(&data) {
                *self.issue_types.write() = issue_types;
            }
        }

        // Load components
        if let Ok(data) = fs::read_to_string(self.data_dir.join("components.json")) {
            if let Ok(components) = serde_json::from_str(&data) {
                *self.components.write() = components;
            }
        }

        // Load versions
        if let Ok(data) = fs::read_to_string(self.data_dir.join("versions.json")) {
            if let Ok(versions) = serde_json::from_str(&data) {
                *self.versions.write() = versions;
            }
        }

        // Load fields
        if let Ok(data) = fs::read_to_string(self.data_dir.join("fields.json")) {
            if let Ok(fields) = serde_json::from_str(&data) {
                *self.fields.write() = fields;
            }
        }

        // Load issue links
        if let Ok(data) = fs::read_to_string(self.data_dir.join("issue_links.json")) {
            if let Ok(links) = serde_json::from_str(&data) {
                *self.issue_links.write() = links;
            }
        }

        // Load transitions
        if let Ok(data) = fs::read_to_string(self.data_dir.join("transitions.json")) {
            if let Ok(transitions) = serde_json::from_str(&data) {
                *self.transitions.write() = transitions;
            }
        }

        Ok(())
    }

    /// Save all data to JSON files
    pub fn save(&self) -> Result<(), String> {
        fs::create_dir_all(&self.data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        self.save_json("projects.json", &*self.projects.read())?;
        self.save_json("issues.json", &*self.issues.read())?;
        self.save_json("statuses.json", &*self.statuses.read())?;
        self.save_json("priorities.json", &*self.priorities.read())?;
        self.save_json("issue_types.json", &*self.issue_types.read())?;
        self.save_json("components.json", &*self.components.read())?;
        self.save_json("versions.json", &*self.versions.read())?;
        self.save_json("fields.json", &*self.fields.read())?;
        self.save_json("issue_links.json", &*self.issue_links.read())?;
        self.save_json("transitions.json", &*self.transitions.read())?;

        Ok(())
    }

    fn save_json<T: Serialize>(&self, filename: &str, data: &T) -> Result<(), String> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize {}: {}", filename, e))?;
        fs::write(self.data_dir.join(filename), json)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))
    }

    /// Save default sample data
    fn save_defaults(&self) -> Result<(), String> {
        // Default project
        *self.projects.write() = vec![Project {
            id: "10000".to_string(),
            key: "TEST".to_string(),
            name: "Test Project".to_string(),
            project_type_key: "software".to_string(),
            simplified: false,
            style: "next-gen".to_string(),
            is_private: false,
            self_url: "http://localhost:8080/rest/api/3/project/10000".to_string(),
        }];

        // Default statuses
        let mut statuses = HashMap::new();
        statuses.insert(
            "TEST".to_string(),
            vec![
                Status {
                    id: "1".to_string(),
                    name: "To Do".to_string(),
                    description: Some("Issue is open and ready to be worked on".to_string()),
                    status_category: StatusCategory {
                        id: 2,
                        key: "new".to_string(),
                        name: "To Do".to_string(),
                        color_name: "blue-gray".to_string(),
                    },
                },
                Status {
                    id: "2".to_string(),
                    name: "In Progress".to_string(),
                    description: Some("Issue is being worked on".to_string()),
                    status_category: StatusCategory {
                        id: 4,
                        key: "indeterminate".to_string(),
                        name: "In Progress".to_string(),
                        color_name: "yellow".to_string(),
                    },
                },
                Status {
                    id: "3".to_string(),
                    name: "Done".to_string(),
                    description: Some("Issue is completed".to_string()),
                    status_category: StatusCategory {
                        id: 3,
                        key: "done".to_string(),
                        name: "Done".to_string(),
                        color_name: "green".to_string(),
                    },
                },
            ],
        );
        *self.statuses.write() = statuses;

        // Default priorities
        *self.priorities.write() = vec![
            Priority {
                id: "1".to_string(),
                name: "Highest".to_string(),
                description: Some("Critical priority".to_string()),
                icon_url: None,
            },
            Priority {
                id: "2".to_string(),
                name: "High".to_string(),
                description: Some("High priority".to_string()),
                icon_url: None,
            },
            Priority {
                id: "3".to_string(),
                name: "Medium".to_string(),
                description: Some("Medium priority".to_string()),
                icon_url: None,
            },
            Priority {
                id: "4".to_string(),
                name: "Low".to_string(),
                description: Some("Low priority".to_string()),
                icon_url: None,
            },
            Priority {
                id: "5".to_string(),
                name: "Lowest".to_string(),
                description: Some("Lowest priority".to_string()),
                icon_url: None,
            },
        ];

        // Default issue types
        let mut issue_types = HashMap::new();
        issue_types.insert(
            "TEST".to_string(),
            vec![
                IssueType {
                    id: "10000".to_string(),
                    name: "Epic".to_string(),
                    description: Some("A big user story".to_string()),
                    icon_url: None,
                    subtask: false,
                    hierarchy_level: Some(0),
                },
                IssueType {
                    id: "10001".to_string(),
                    name: "Story".to_string(),
                    description: Some("A user story".to_string()),
                    icon_url: None,
                    subtask: false,
                    hierarchy_level: Some(1),
                },
                IssueType {
                    id: "10002".to_string(),
                    name: "Task".to_string(),
                    description: Some("A task that needs to be done".to_string()),
                    icon_url: None,
                    subtask: false,
                    hierarchy_level: Some(1),
                },
                IssueType {
                    id: "10003".to_string(),
                    name: "Bug".to_string(),
                    description: Some("A bug or defect".to_string()),
                    icon_url: None,
                    subtask: false,
                    hierarchy_level: Some(1),
                },
            ],
        );
        *self.issue_types.write() = issue_types;

        // Default transitions
        let mut transitions = HashMap::new();
        transitions.insert(
            "To Do".to_string(),
            vec![Transition {
                id: "21".to_string(),
                name: "Start Progress".to_string(),
                to: TransitionTo {
                    id: "2".to_string(),
                    name: "In Progress".to_string(),
                },
            }],
        );
        transitions.insert(
            "In Progress".to_string(),
            vec![
                Transition {
                    id: "31".to_string(),
                    name: "Done".to_string(),
                    to: TransitionTo {
                        id: "3".to_string(),
                        name: "Done".to_string(),
                    },
                },
                Transition {
                    id: "11".to_string(),
                    name: "Stop Progress".to_string(),
                    to: TransitionTo {
                        id: "1".to_string(),
                        name: "To Do".to_string(),
                    },
                },
            ],
        );
        transitions.insert(
            "Done".to_string(),
            vec![Transition {
                id: "41".to_string(),
                name: "Reopen".to_string(),
                to: TransitionTo {
                    id: "1".to_string(),
                    name: "To Do".to_string(),
                },
            }],
        );
        *self.transitions.write() = transitions;

        // Default fields
        *self.fields.write() = vec![
            Field {
                id: "summary".to_string(),
                name: "Summary".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "description".to_string(),
                name: "Description".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "status".to_string(),
                name: "Status".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "priority".to_string(),
                name: "Priority".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "issuetype".to_string(),
                name: "Issue Type".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "assignee".to_string(),
                name: "Assignee".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "reporter".to_string(),
                name: "Reporter".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "created".to_string(),
                name: "Created".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "updated".to_string(),
                name: "Updated".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "duedate".to_string(),
                name: "Due Date".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "labels".to_string(),
                name: "Labels".to_string(),
                custom: false,
                orderable: true,
                navigable: true,
                searchable: true,
                schema: None,
            },
            Field {
                id: "parent".to_string(),
                name: "Parent".to_string(),
                custom: false,
                orderable: false,
                navigable: true,
                searchable: true,
                schema: None,
            },
        ];

        self.save()
    }

    /// Get next issue number for a project
    pub fn next_issue_number(&self, project_key: &str) -> u64 {
        let issues = self.issues.read();
        let prefix = format!("{}-", project_key);
        let max_num = issues
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter_map(|n| n.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        max_num + 1
    }
}

pub type SharedDataStore = Arc<DataStore>;

// ============================================================
// Data Models (JIRA API compatible)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
    pub project_type_key: String,
    pub simplified: bool,
    pub style: String,
    pub is_private: bool,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status_category: StatusCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCategory {
    pub id: i32,
    pub key: String,
    pub name: String,
    pub color_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub subtask: bool,
    pub hierarchy_level: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "self")]
    pub self_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub released: bool,
    pub release_date: Option<String>,
    #[serde(rename = "self")]
    pub self_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub orderable: bool,
    pub navigable: bool,
    pub searchable: bool,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub fields: IssueFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<Changelog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueFields {
    pub summary: String,
    pub description: Option<serde_json::Value>,
    pub issuetype: IssueTypeRef,
    pub project: ProjectRef,
    pub status: StatusRef,
    pub priority: Option<PriorityRef>,
    pub assignee: Option<UserRef>,
    pub reporter: Option<UserRef>,
    pub created: String,
    pub updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duedate: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ParentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ComponentRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_versions: Option<Vec<VersionRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTypeRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    pub id: String,
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusRef {
    pub id: String,
    pub name: String,
    pub status_category: StatusCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRef {
    pub account_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentRef {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Changelog {
    pub start_at: i32,
    pub max_results: i32,
    pub total: i32,
    pub histories: Vec<ChangelogHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogHistory {
    pub id: String,
    pub author: UserRef,
    pub created: String,
    pub items: Vec<ChangelogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogItem {
    pub field: String,
    pub field_type: String,
    pub from: Option<String>,
    pub from_string: Option<String>,
    pub to: Option<String>,
    pub to_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLink {
    pub id: String,
    #[serde(rename = "type")]
    pub link_type: IssueLinkType,
    pub inward_issue: IssueLinkRef,
    pub outward_issue: IssueLinkRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkType {
    pub id: String,
    pub name: String,
    pub inward: String,
    pub outward: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkRef {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub name: String,
    pub to: TransitionTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionTo {
    pub id: String,
    pub name: String,
}

// ============================================================
// API Request/Response types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub start_at: i32,
    pub max_results: i32,
    pub total: i32,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueRequest {
    pub fields: CreateIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueFields {
    pub project: ProjectKeyRef,
    pub summary: String,
    pub issuetype: IssueTypeNameRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityNameRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ParentKeyRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectKeyRef {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTypeNameRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityNameRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentKeyRef {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueResponse {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionsResponse {
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoTransitionRequest {
    pub transition: TransitionIdRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionIdRef {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueLinkRequest {
    #[serde(rename = "type")]
    pub link_type: IssueLinkTypeNameRef,
    pub inward_issue: IssueKeyRef,
    pub outward_issue: IssueKeyRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkTypeNameRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueKeyRef {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    pub fields: UpdateIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duedate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(rename = "errorMessages")]
    pub error_messages: Vec<String>,
    pub errors: HashMap<String, String>,
}
