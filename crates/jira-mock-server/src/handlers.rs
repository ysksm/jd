//! API handlers for the mock JIRA server

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::data::*;

// ============================================================
// Projects
// ============================================================

pub async fn get_projects(State(store): State<SharedDataStore>) -> impl IntoResponse {
    let projects = store.projects.read().clone();
    Json(projects)
}

// ============================================================
// Search Issues
// ============================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SearchQuery {
    pub jql: Option<String>,
    pub fields: Option<String>,
    pub expand: Option<String>,
    #[serde(rename = "maxResults")]
    pub max_results: Option<i32>,
    #[serde(rename = "startAt")]
    pub start_at: Option<i32>,
}

pub async fn search_issues_get(
    State(store): State<SharedDataStore>,
    Query(query): Query<SearchQuery>,
) -> axum::response::Response {
    search_issues_internal(&store, query).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SearchBody {
    pub jql: Option<String>,
    pub fields: Option<Vec<String>>,
    pub expand: Option<Vec<String>>,
    #[serde(rename = "maxResults")]
    pub max_results: Option<i32>,
    #[serde(rename = "startAt")]
    pub start_at: Option<i32>,
}

pub async fn search_issues_post(
    State(store): State<SharedDataStore>,
    Json(body): Json<SearchBody>,
) -> axum::response::Response {
    let query = SearchQuery {
        jql: body.jql,
        fields: body.fields.map(|f| f.join(",")),
        expand: body.expand.map(|e| e.join(",")),
        max_results: body.max_results,
        start_at: body.start_at,
    };
    search_issues_internal(&store, query).into_response()
}

fn search_issues_internal(store: &SharedDataStore, query: SearchQuery) -> Json<SearchResponse> {
    let issues = store.issues.read();
    let jql = query.jql.unwrap_or_default();
    let max_results = query.max_results.unwrap_or(50);
    let start_at = query.start_at.unwrap_or(0);
    let include_changelog = query
        .expand
        .as_ref()
        .map(|e| e.contains("changelog"))
        .unwrap_or(false);

    // Simple JQL parsing (project = KEY)
    let filtered: Vec<Issue> = issues
        .values()
        .filter(|issue| {
            if jql.is_empty() {
                return true;
            }
            // Simple project filter
            if let Some(project_key) = extract_project_from_jql(&jql) {
                if issue.fields.project.key != project_key {
                    return false;
                }
            }
            // Simple text search
            if let Some(text) = extract_text_search_from_jql(&jql) {
                let summary_lower = issue.fields.summary.to_lowercase();
                if !summary_lower.contains(&text.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .map(|mut issue| {
            if !include_changelog {
                issue.changelog = None;
            }
            issue
        })
        .collect();

    let total = filtered.len() as i32;
    let start = start_at as usize;
    let end = std::cmp::min(start + max_results as usize, filtered.len());
    let page_issues = if start < filtered.len() {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    Json(SearchResponse {
        start_at,
        max_results,
        total,
        issues: page_issues,
    })
}

fn extract_project_from_jql(jql: &str) -> Option<String> {
    // Simple pattern: project = KEY or project = "KEY"
    let jql_lower = jql.to_lowercase();
    if let Some(pos) = jql_lower.find("project") {
        let rest = &jql[pos..];
        if let Some(eq_pos) = rest.find('=') {
            let after_eq = rest[eq_pos + 1..].trim();
            let key: String = after_eq
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !key.is_empty() {
                return Some(key.trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_text_search_from_jql(jql: &str) -> Option<String> {
    // Simple pattern: text ~ "search term"
    let jql_lower = jql.to_lowercase();
    if let Some(pos) = jql_lower.find("text") {
        let rest = &jql[pos..];
        if let Some(tilde_pos) = rest.find('~') {
            let after_tilde = rest[tilde_pos + 1..].trim();
            if after_tilde.starts_with('"') {
                let end_quote = after_tilde[1..].find('"').map(|p| p + 1);
                if let Some(end) = end_quote {
                    return Some(after_tilde[1..end].to_string());
                }
            }
        }
    }
    None
}

// ============================================================
// Project Metadata
// ============================================================

pub async fn get_project_statuses(
    State(store): State<SharedDataStore>,
    Path(project_key): Path<String>,
) -> impl IntoResponse {
    let statuses = store.statuses.read();
    if let Some(project_statuses) = statuses.get(&project_key) {
        // Return in JIRA's format: array of issue types with statuses
        let response: Vec<serde_json::Value> = store
            .issue_types
            .read()
            .get(&project_key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|it| {
                serde_json::json!({
                    "self": format!("http://localhost:8080/rest/api/3/issuetype/{}", it.id),
                    "id": it.id,
                    "name": it.name,
                    "subtask": it.subtask,
                    "statuses": project_statuses
                })
            })
            .collect();
        (StatusCode::OK, Json(response)).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error_messages: vec!["Project not found".to_string()],
                errors: HashMap::new(),
            }),
        )
            .into_response()
    }
}

pub async fn get_priorities(State(store): State<SharedDataStore>) -> impl IntoResponse {
    let priorities = store.priorities.read().clone();
    Json(priorities)
}

#[derive(Debug, Deserialize)]
pub struct IssueTypeQuery {
    #[serde(rename = "projectId")]
    pub project_id: Option<String>,
}

pub async fn get_issue_types(
    State(store): State<SharedDataStore>,
    Query(query): Query<IssueTypeQuery>,
) -> impl IntoResponse {
    // Find project key by ID
    let project_key = if let Some(project_id) = query.project_id {
        let projects = store.projects.read();
        projects
            .iter()
            .find(|p| p.id == project_id)
            .map(|p| p.key.clone())
    } else {
        None
    };

    if let Some(key) = project_key {
        let issue_types = store.issue_types.read();
        if let Some(types) = issue_types.get(&key) {
            return Json(types.clone()).into_response();
        }
    }

    // Return empty array if not found
    Json(Vec::<IssueType>::new()).into_response()
}

pub async fn get_issue_types_by_project_key(
    State(store): State<SharedDataStore>,
    Path(project_key): Path<String>,
) -> impl IntoResponse {
    let issue_types = store.issue_types.read();
    if let Some(types) = issue_types.get(&project_key) {
        let response: Vec<serde_json::Value> = types
            .iter()
            .map(|it| {
                serde_json::json!({
                    "self": format!("http://localhost:8080/rest/api/3/issuetype/{}", it.id),
                    "id": it.id,
                    "name": it.name,
                    "description": it.description,
                    "iconUrl": it.icon_url,
                    "subtask": it.subtask,
                    "hierarchyLevel": it.hierarchy_level
                })
            })
            .collect();
        Json(serde_json::json!({ "issueTypes": response })).into_response()
    } else {
        Json(serde_json::json!({ "issueTypes": [] })).into_response()
    }
}

pub async fn get_components(
    State(store): State<SharedDataStore>,
    Path(project_key): Path<String>,
) -> impl IntoResponse {
    let components = store.components.read();
    if let Some(comps) = components.get(&project_key) {
        Json(comps.clone()).into_response()
    } else {
        Json(Vec::<Component>::new()).into_response()
    }
}

pub async fn get_versions(
    State(store): State<SharedDataStore>,
    Path(project_key): Path<String>,
) -> impl IntoResponse {
    let versions = store.versions.read();
    if let Some(vers) = versions.get(&project_key) {
        Json(vers.clone()).into_response()
    } else {
        Json(Vec::<Version>::new()).into_response()
    }
}

pub async fn get_fields(State(store): State<SharedDataStore>) -> impl IntoResponse {
    let fields = store.fields.read().clone();
    Json(fields)
}

// ============================================================
// Issue CRUD
// ============================================================

pub async fn create_issue(
    State(store): State<SharedDataStore>,
    Json(request): Json<CreateIssueRequest>,
) -> impl IntoResponse {
    let project_key = &request.fields.project.key;

    // Validate project exists
    {
        let projects = store.projects.read();
        if !projects.iter().any(|p| &p.key == project_key) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error_messages: vec![],
                    errors: {
                        let mut e = HashMap::new();
                        e.insert("project".to_string(), "Project not found".to_string());
                        e
                    },
                }),
            )
                .into_response();
        }
    }

    // Validate issue type
    let issue_type = {
        let issue_types = store.issue_types.read();
        let types = issue_types.get(project_key);
        types.and_then(|t| {
            t.iter()
                .find(|it| it.name == request.fields.issuetype.name)
                .cloned()
        })
    };

    let issue_type = match issue_type {
        Some(it) => it,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error_messages: vec![],
                    errors: {
                        let mut e = HashMap::new();
                        e.insert(
                            "issuetype".to_string(),
                            "有効な課題タイプを指定してください".to_string(),
                        );
                        e
                    },
                }),
            )
                .into_response();
        }
    };

    // Get project info
    let project = {
        let projects = store.projects.read();
        projects.iter().find(|p| &p.key == project_key).cloned()
    };
    let project = project.unwrap();

    // Get default status
    let default_status = {
        let statuses = store.statuses.read();
        statuses
            .get(project_key)
            .and_then(|s| s.first().cloned())
            .unwrap_or_else(|| Status {
                id: "1".to_string(),
                name: "To Do".to_string(),
                description: None,
                status_category: StatusCategory {
                    id: 2,
                    key: "new".to_string(),
                    name: "To Do".to_string(),
                    color_name: "blue-gray".to_string(),
                },
            })
    };

    // Get priority
    let priority = request.fields.priority.as_ref().and_then(|p| {
        let priorities = store.priorities.read();
        priorities.iter().find(|pr| pr.name == p.name).cloned()
    });

    let issue_number = store.next_issue_number(project_key);
    let issue_key = format!("{}-{}", project_key, issue_number);
    let issue_id = format!("{}", 10000 + issue_number);
    let now = Utc::now().to_rfc3339();

    let issue = Issue {
        id: issue_id.clone(),
        key: issue_key.clone(),
        self_url: format!("http://localhost:8080/rest/api/3/issue/{}", issue_id),
        fields: IssueFields {
            summary: request.fields.summary,
            description: request.fields.description,
            issuetype: IssueTypeRef {
                id: issue_type.id,
                name: issue_type.name,
            },
            project: ProjectRef {
                id: project.id,
                key: project.key,
                name: project.name,
            },
            status: StatusRef {
                id: default_status.id,
                name: default_status.name,
                status_category: default_status.status_category,
            },
            priority: priority.map(|p| PriorityRef {
                id: p.id,
                name: p.name,
            }),
            assignee: None,
            reporter: None,
            created: now.clone(),
            updated: now,
            duedate: None,
            labels: request.fields.labels.unwrap_or_default(),
            parent: request.fields.parent.map(|p| ParentRef {
                id: "0".to_string(),
                key: p.key,
            }),
            components: None,
            fix_versions: None,
        },
        changelog: Some(Changelog {
            start_at: 0,
            max_results: 0,
            total: 0,
            histories: vec![],
        }),
    };

    // Save issue
    {
        let mut issues = store.issues.write();
        issues.insert(issue_key.clone(), issue);
    }

    // Persist to disk
    if let Err(e) = store.save() {
        tracing::error!("Failed to save data: {}", e);
    }

    (
        StatusCode::CREATED,
        Json(CreateIssueResponse {
            id: issue_id,
            key: issue_key.clone(),
            self_url: format!("http://localhost:8080/rest/api/3/issue/{}", issue_key),
        }),
    )
        .into_response()
}

pub async fn update_issue(
    State(store): State<SharedDataStore>,
    Path(issue_key): Path<String>,
    Json(request): Json<UpdateIssueRequest>,
) -> impl IntoResponse {
    let mut issues = store.issues.write();

    if let Some(issue) = issues.get_mut(&issue_key) {
        if let Some(duedate) = request.fields.duedate {
            issue.fields.duedate = Some(duedate);
        }
        if let Some(summary) = request.fields.summary {
            issue.fields.summary = summary;
        }
        if let Some(description) = request.fields.description {
            issue.fields.description = Some(description);
        }
        issue.fields.updated = Utc::now().to_rfc3339();

        drop(issues);

        // Persist to disk
        if let Err(e) = store.save() {
            tracing::error!("Failed to save data: {}", e);
        }

        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error_messages: vec!["Issue not found".to_string()],
                errors: HashMap::new(),
            }),
        )
            .into_response()
    }
}

// ============================================================
// Transitions
// ============================================================

pub async fn get_transitions(
    State(store): State<SharedDataStore>,
    Path(issue_key): Path<String>,
) -> impl IntoResponse {
    let issues = store.issues.read();

    if let Some(issue) = issues.get(&issue_key) {
        let current_status = &issue.fields.status.name;
        let transitions = store.transitions.read();

        let available = transitions.get(current_status).cloned().unwrap_or_default();

        Json(TransitionsResponse {
            transitions: available,
        })
        .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error_messages: vec!["Issue not found".to_string()],
                errors: HashMap::new(),
            }),
        )
            .into_response()
    }
}

pub async fn do_transition(
    State(store): State<SharedDataStore>,
    Path(issue_key): Path<String>,
    Json(request): Json<DoTransitionRequest>,
) -> impl IntoResponse {
    let mut issues = store.issues.write();

    if let Some(issue) = issues.get_mut(&issue_key) {
        let current_status = issue.fields.status.name.clone();
        let transitions = store.transitions.read();

        let transition = transitions
            .get(&current_status)
            .and_then(|ts| ts.iter().find(|t| t.id == request.transition.id));

        if let Some(t) = transition {
            // Update status
            issue.fields.status = StatusRef {
                id: t.to.id.clone(),
                name: t.to.name.clone(),
                status_category: StatusCategory {
                    id: match t.to.name.as_str() {
                        "To Do" => 2,
                        "In Progress" => 4,
                        "Done" => 3,
                        _ => 2,
                    },
                    key: match t.to.name.as_str() {
                        "To Do" => "new".to_string(),
                        "In Progress" => "indeterminate".to_string(),
                        "Done" => "done".to_string(),
                        _ => "new".to_string(),
                    },
                    name: t.to.name.clone(),
                    color_name: match t.to.name.as_str() {
                        "To Do" => "blue-gray".to_string(),
                        "In Progress" => "yellow".to_string(),
                        "Done" => "green".to_string(),
                        _ => "blue-gray".to_string(),
                    },
                },
            };
            issue.fields.updated = Utc::now().to_rfc3339();

            // Add to changelog
            if let Some(changelog) = &mut issue.changelog {
                changelog.histories.push(ChangelogHistory {
                    id: Uuid::new_v4().to_string(),
                    author: UserRef {
                        account_id: "mock-user".to_string(),
                        display_name: "Mock User".to_string(),
                        email_address: None,
                    },
                    created: Utc::now().to_rfc3339(),
                    items: vec![ChangelogItem {
                        field: "status".to_string(),
                        field_type: "jira".to_string(),
                        from: Some(current_status.clone()),
                        from_string: Some(current_status),
                        to: Some(t.to.id.clone()),
                        to_string: Some(t.to.name.clone()),
                    }],
                });
                changelog.total = changelog.histories.len() as i32;
            }

            drop(issues);
            drop(transitions);

            // Persist to disk
            if let Err(e) = store.save() {
                tracing::error!("Failed to save data: {}", e);
            }

            StatusCode::NO_CONTENT.into_response()
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error_messages: vec!["Invalid transition".to_string()],
                    errors: HashMap::new(),
                }),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error_messages: vec!["Issue not found".to_string()],
                errors: HashMap::new(),
            }),
        )
            .into_response()
    }
}

// ============================================================
// Issue Links
// ============================================================

pub async fn create_issue_link(
    State(store): State<SharedDataStore>,
    Json(request): Json<CreateIssueLinkRequest>,
) -> impl IntoResponse {
    // Validate issues exist
    {
        let issues = store.issues.read();
        if !issues.contains_key(&request.inward_issue.key) {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error_messages: vec![format!("Issue {} not found", request.inward_issue.key)],
                    errors: HashMap::new(),
                }),
            )
                .into_response();
        }
        if !issues.contains_key(&request.outward_issue.key) {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error_messages: vec![format!("Issue {} not found", request.outward_issue.key)],
                    errors: HashMap::new(),
                }),
            )
                .into_response();
        }
    }

    let link = IssueLink {
        id: Uuid::new_v4().to_string(),
        link_type: IssueLinkType {
            id: "1".to_string(),
            name: request.link_type.name.clone(),
            inward: match request.link_type.name.as_str() {
                "Blocks" => "is blocked by".to_string(),
                "Relates" => "relates to".to_string(),
                _ => "relates to".to_string(),
            },
            outward: match request.link_type.name.as_str() {
                "Blocks" => "blocks".to_string(),
                "Relates" => "relates to".to_string(),
                _ => "relates to".to_string(),
            },
        },
        inward_issue: IssueLinkRef {
            id: "0".to_string(),
            key: request.inward_issue.key,
        },
        outward_issue: IssueLinkRef {
            id: "0".to_string(),
            key: request.outward_issue.key,
        },
    };

    {
        let mut links = store.issue_links.write();
        links.push(link);
    }

    // Persist to disk
    if let Err(e) = store.save() {
        tracing::error!("Failed to save data: {}", e);
    }

    StatusCode::CREATED.into_response()
}
