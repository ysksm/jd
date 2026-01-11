use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use log::{debug, info, warn};
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::application::dto::{CreatedIssueDto, TransitionDto};
use crate::application::services::{FetchProgress, JiraService};
use crate::domain::entities::{
    Component, FixVersion, Issue, IssueType, JiraField, Label, Priority, Project, Status,
};
use crate::domain::error::{DomainError, DomainResult};
use crate::infrastructure::config::JiraConfig;
use chrono::{DateTime, Utc};

/// Parse JIRA date string which can be in multiple formats:
/// - RFC3339: "2024-01-15T10:30:00.000+00:00"
/// - JIRA format: "2024-01-15T10:30:00.000+0000" (no colon in timezone)
fn parse_jira_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        debug!("Parsed date as RFC3339: {} -> {}", s, dt);
        return Some(dt.with_timezone(&Utc));
    }

    // Try JIRA format (insert colon in timezone offset)
    // "2024-01-15T10:30:00.000+0000" -> "2024-01-15T10:30:00.000+00:00"
    if s.len() >= 5 {
        let len = s.len();
        // Check if it ends with a timezone without colon like +0000 or -0530
        let last5 = &s[len.saturating_sub(5)..];
        if (last5.starts_with('+') || last5.starts_with('-'))
            && last5[1..].chars().all(|c| c.is_ascii_digit())
        {
            let mut fixed = s[..len - 2].to_string();
            fixed.push(':');
            fixed.push_str(&s[len - 2..]);
            if let Ok(dt) = DateTime::parse_from_rfc3339(&fixed) {
                debug!("Parsed date as JIRA format: {} -> {} -> {}", s, fixed, dt);
                return Some(dt.with_timezone(&Utc));
            }
        }
    }

    warn!("Failed to parse JIRA datetime: {}", s);
    None
}

pub struct JiraApiClient {
    client: jira_api::JiraClient,
    http_client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

async fn retry_with_backoff<F, Fut, T, E>(
    mut f: F,
    max_retries: u32,
    timeout_secs: u64,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    T: std::fmt::Debug,
    E: std::fmt::Display + std::fmt::Debug,
{
    let mut retry_count = 0;
    let timeout_duration = Duration::from_secs(timeout_secs);

    loop {
        let result = match timeout(timeout_duration, f()).await {
            Ok(result) => result,
            Err(_) => {
                warn!("Operation timed out after {} seconds", timeout_secs);
                if retry_count >= max_retries {
                    return Err(f().await.unwrap_err());
                }
                retry_count += 1;
                let delay = Duration::from_secs(2u64.pow(retry_count));
                warn!(
                    "Retrying in {:?} (attempt {}/{})",
                    delay, retry_count, max_retries
                );
                sleep(delay).await;
                continue;
            }
        };

        match result {
            Ok(value) => return Ok(value),
            Err(e) => {
                if retry_count >= max_retries {
                    return Err(e);
                }

                retry_count += 1;
                let delay = Duration::from_secs(2u64.pow(retry_count));
                warn!(
                    "Request failed: {}. Retrying in {:?} (attempt {}/{})",
                    e, delay, retry_count, max_retries
                );
                sleep(delay).await;
            }
        }
    }
}

impl JiraApiClient {
    pub fn new(config: &JiraConfig) -> DomainResult<Self> {
        let client = jira_api::JiraClient::new(
            config.endpoint.clone(),
            config.username.clone(),
            config.api_key.clone(),
        );

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create HTTP client: {}", e))
            })?;

        let credentials = format!("{}:{}", config.username, config.api_key);
        let auth_header = format!("Basic {}", general_purpose::STANDARD.encode(credentials));

        Ok(Self {
            client,
            http_client,
            base_url: config.endpoint.clone(),
            auth_header,
        })
    }

    /// Extract sprint name from JIRA fields
    /// Sprint is typically stored in a custom field (customfield_XXXXX)
    /// The format can be either:
    /// - An array of sprint objects with "name" field
    /// - A string in format "com.atlassian.greenhopper.service.sprint.Sprint@xxx[name=Sprint Name,...]"
    fn extract_sprint(fields: &serde_json::Value) -> Option<String> {
        // Common sprint custom field IDs
        let sprint_field_ids = [
            "sprint",
            "customfield_10020", // Common default
            "customfield_10104",
            "customfield_10000",
        ];

        for field_id in sprint_field_ids {
            if let Some(sprint_value) = fields.get(field_id) {
                // Case 1: Array of sprint objects
                if let Some(sprints) = sprint_value.as_array() {
                    // Get the most recent active sprint (last in array is usually most recent)
                    for sprint in sprints.iter().rev() {
                        if let Some(name) = sprint.get("name").and_then(|n| n.as_str()) {
                            // Check if sprint is active (state = "active") or just return name
                            let state = sprint.get("state").and_then(|s| s.as_str()).unwrap_or("");
                            if state == "active" || state == "closed" || state.is_empty() {
                                return Some(name.to_string());
                            }
                        }
                    }
                    // If no active sprint found, return the first one with a name
                    for sprint in sprints {
                        if let Some(name) = sprint.get("name").and_then(|n| n.as_str()) {
                            return Some(name.to_string());
                        }
                    }
                }

                // Case 2: String format (legacy)
                if let Some(sprint_str) = sprint_value.as_str() {
                    // Parse "name=Sprint Name" from the string
                    if let Some(name_start) = sprint_str.find("name=") {
                        let name_portion = &sprint_str[name_start + 5..];
                        if let Some(comma_pos) = name_portion.find(',') {
                            return Some(name_portion[..comma_pos].to_string());
                        } else if let Some(bracket_pos) = name_portion.find(']') {
                            return Some(name_portion[..bracket_pos].to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract team name from JIRA fields
    /// Team is typically stored in a custom field (customfield_XXXXX)
    /// The format can be:
    /// - An object with "name" or "value" field
    /// - A string directly
    fn extract_team(fields: &serde_json::Value) -> Option<String> {
        // Common team custom field IDs
        let team_field_ids = [
            "team",
            "customfield_10001", // Common default for team
            "customfield_10002",
            "customfield_10100",
            "customfield_10101",
        ];

        for field_id in team_field_ids {
            if let Some(team_value) = fields.get(field_id) {
                // Case 1: Object with "name" field (e.g., {"name": "Team A", "id": "123"})
                if let Some(name) = team_value.get("name").and_then(|n| n.as_str()) {
                    return Some(name.to_string());
                }

                // Case 2: Object with "value" field (e.g., {"value": "Team A"})
                if let Some(value) = team_value.get("value").and_then(|v| v.as_str()) {
                    return Some(value.to_string());
                }

                // Case 3: String directly
                if let Some(team_str) = team_value.as_str() {
                    if !team_str.is_empty() {
                        return Some(team_str.to_string());
                    }
                }
            }
        }

        None
    }

    /// Parse a single issue from JSON response
    fn parse_issue(issue_json: &serde_json::Value) -> Option<Issue> {
        let id = issue_json["id"].as_str()?;
        let key = issue_json["key"].as_str()?;
        let fields = &issue_json["fields"];

        let project_id = fields["project"]["id"].as_str().unwrap_or("").to_string();
        let summary = fields["summary"].as_str().unwrap_or("").to_string();
        let description = fields["description"].as_str().map(|s| s.to_string());
        let status = fields["status"]["name"].as_str().map(|s| s.to_string());
        let priority = fields["priority"]["name"].as_str().map(|s| s.to_string());
        let assignee = fields["assignee"]["displayName"]
            .as_str()
            .map(|s| s.to_string());
        let reporter = fields["reporter"]["displayName"]
            .as_str()
            .map(|s| s.to_string());
        let issue_type = fields["issuetype"]["name"].as_str().map(|s| s.to_string());
        let resolution = fields["resolution"]["name"].as_str().map(|s| s.to_string());

        let labels = fields["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        let components = fields["components"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        let fix_versions = fields["fixVersions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        let sprint = Self::extract_sprint(fields);
        let team = Self::extract_team(fields);
        let parent_key = fields["parent"]["key"].as_str().map(|s| s.to_string());

        // Parse due date (JIRA returns date as "YYYY-MM-DD" string)
        let due_date = fields["duedate"]
            .as_str()
            .and_then(|s| {
                // JIRA returns due date as "YYYY-MM-DD" without time
                // Convert to DateTime by appending "T00:00:00Z"
                let datetime_str = format!("{}T00:00:00Z", s);
                chrono::DateTime::parse_from_rfc3339(&datetime_str).ok()
            })
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let created_str = fields["created"].as_str();
        let updated_str = fields["updated"].as_str();
        debug!(
            "[parse_issue] Issue {} date strings: created={:?}, updated={:?}",
            key, created_str, updated_str
        );

        let created_date = created_str.and_then(parse_jira_datetime);
        let updated_date = updated_str.and_then(parse_jira_datetime);
        debug!(
            "[parse_issue] Issue {} parsed dates: created_date={:?}, updated_date={:?}",
            key, created_date, updated_date
        );

        let raw_json = serde_json::to_string(&issue_json).ok();

        Some(Issue::new(
            id.to_string(),
            project_id,
            key.to_string(),
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
            team,
            parent_key,
            due_date,
            created_date,
            updated_date,
            raw_json,
        ))
    }
}

#[async_trait]
impl JiraService for JiraApiClient {
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>> {
        let client = &self.client;

        let projects = retry_with_backoff(|| async { jira_api::get_projects(client).await }, 3, 30)
            .await
            .map_err(|e| DomainError::ExternalService(e.to_string()))?;

        Ok(projects
            .into_iter()
            .map(|p| Project::new(p.id, p.key, p.name, p.description))
            .collect())
    }

    async fn fetch_project_issues(&self, project_key: &str) -> DomainResult<Vec<Issue>> {
        // Use the batch method to fetch all issues with token-based pagination
        let mut all_issues = Vec::new();
        let mut page_token: Option<String> = None;
        let max_results = 100;

        loop {
            let progress = self
                .fetch_project_issues_batch(project_key, None, page_token.as_deref(), max_results)
                .await?;

            all_issues.extend(progress.issues);

            if !progress.has_more {
                break;
            }
            page_token = progress.next_page_token;
        }

        Ok(all_issues)
    }

    async fn fetch_project_issues_batch(
        &self,
        project_key: &str,
        after_updated_at: Option<DateTime<Utc>>,
        page_token: Option<&str>,
        max_results: usize,
    ) -> DomainResult<FetchProgress> {
        // Build JQL: order by updated ASC (oldest first) for resumable sync
        let jql = if let Some(after) = after_updated_at {
            let formatted_date = after.format("%Y-%m-%d %H:%M").to_string();
            info!(
                "[JIRA API] Incremental sync: after_updated_at={:?}, formatted={}",
                after, formatted_date
            );
            format!(
                "project = {} AND updated >= \"{}\" ORDER BY updated ASC, key ASC",
                project_key, formatted_date
            )
        } else {
            format!("project = {} ORDER BY updated ASC, key ASC", project_key)
        };

        info!("[JIRA API] JQL query: {}", jql);

        // Use GET /rest/api/3/search/jql with token-based pagination
        let url = format!("{}/rest/api/3/search/jql", self.base_url);

        debug!(
            "[JIRA API] GET {} (jql={}, pageToken={:?}, maxResults={})",
            url, jql, page_token, max_results
        );

        // Build query parameters
        // Note: *navigable may not include created/updated fields in all JIRA configurations,
        // so we explicitly request them to ensure they are always returned.
        let mut query_params: Vec<(&str, String)> = vec![
            ("jql", jql.clone()),
            ("fields", "*navigable,created,updated".to_string()),
            ("expand", "changelog".to_string()),
            ("maxResults", max_results.to_string()),
        ];

        if let Some(token) = page_token {
            query_params.push(("nextPageToken", token.to_string()));
        }

        let response = self
            .http_client
            .get(&url)
            .query(&query_params)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to fetch issues: {}", e)))?;

        debug!("[JIRA API] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch issues: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to parse issues: {}", e)))?;

        // Parse token-based pagination fields
        let is_last = json["isLast"].as_bool().unwrap_or(true);
        let next_page_token = json["nextPageToken"].as_str().map(|s| s.to_string());

        let mut issues = Vec::new();
        if let Some(issues_array) = json["issues"].as_array() {
            for issue_json in issues_array {
                if let Some(issue) = Self::parse_issue(issue_json) {
                    issues.push(issue);
                }
            }
        }

        let has_more = !is_last && next_page_token.is_some();

        info!(
            "[JIRA API] Fetched {} issues, isLast={}, has_more={}",
            issues.len(),
            is_last,
            has_more
        );

        Ok(FetchProgress {
            issues,
            total: 0,          // Token-based pagination doesn't provide total
            fetched_so_far: 0, // Will be calculated by caller
            has_more,
            next_page_token,
        })
    }

    async fn test_connection(&self) -> DomainResult<()> {
        let client = &self.client;

        retry_with_backoff(|| async { jira_api::get_projects(client).await }, 2, 15)
            .await
            .map_err(|e| DomainError::ExternalService(format!("Connection test failed: {}", e)))?;

        Ok(())
    }

    async fn fetch_project_statuses(&self, project_key: &str) -> DomainResult<Vec<Status>> {
        let url = format!(
            "{}/rest/api/3/project/{}/statuses",
            self.base_url, project_key
        );

        debug!("[JIRA API] GET {} (fetching statuses)", url);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch statuses: {}", e))
            })?;

        debug!("[JIRA API] Response status: {}", response.status());

        if !response.status().is_success() {
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch statuses: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse statuses: {}", e))
        })?;

        let mut statuses = Vec::new();
        if let Some(issue_types) = json.as_array() {
            for issue_type in issue_types {
                if let Some(status_array) = issue_type["statuses"].as_array() {
                    for status_obj in status_array {
                        if let Some(name) = status_obj["name"].as_str() {
                            statuses.push(Status {
                                name: name.to_string(),
                                description: status_obj["description"]
                                    .as_str()
                                    .map(|s| s.to_string()),
                                category: status_obj["statusCategory"]["key"]
                                    .as_str()
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(statuses)
    }

    async fn fetch_priorities(&self) -> DomainResult<Vec<Priority>> {
        let url = format!("{}/rest/api/3/priority", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch priorities: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch priorities: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse priorities: {}", e))
        })?;

        let mut priorities = Vec::new();
        if let Some(priority_array) = json.as_array() {
            for priority_obj in priority_array {
                if let Some(name) = priority_obj["name"].as_str() {
                    priorities.push(Priority {
                        name: name.to_string(),
                        description: priority_obj["description"].as_str().map(|s| s.to_string()),
                        icon_url: priority_obj["iconUrl"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(priorities)
    }

    async fn fetch_project_issue_types(&self, project_id: &str) -> DomainResult<Vec<IssueType>> {
        let url = format!(
            "{}/rest/api/3/issuetype/project?projectId={}",
            self.base_url, project_id
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch issue types: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch issue types: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse issue types: {}", e))
        })?;

        let mut issue_types = Vec::new();
        if let Some(type_array) = json.as_array() {
            for type_obj in type_array {
                if let Some(name) = type_obj["name"].as_str() {
                    issue_types.push(IssueType {
                        name: name.to_string(),
                        description: type_obj["description"].as_str().map(|s| s.to_string()),
                        icon_url: type_obj["iconUrl"].as_str().map(|s| s.to_string()),
                        subtask: type_obj["subtask"].as_bool().unwrap_or(false),
                    });
                }
            }
        }

        Ok(issue_types)
    }

    async fn fetch_issue_types_by_project_key(
        &self,
        project_key: &str,
    ) -> DomainResult<Vec<IssueType>> {
        // Use createmeta endpoint which accepts project key
        let url = format!(
            "{}/rest/api/3/issue/createmeta/{}/issuetypes",
            self.base_url, project_key
        );

        debug!(
            "[JIRA API] GET {} (fetching issue types for {})",
            url, project_key
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch issue types: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch issue types: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse issue types: {}", e))
        })?;

        let mut issue_types = Vec::new();

        // The response has { "issueTypes": [...] } structure
        let types_array = json
            .get("issueTypes")
            .or_else(|| json.get("values"))
            .and_then(|v| v.as_array());

        if let Some(type_array) = types_array {
            for type_obj in type_array {
                if let Some(name) = type_obj["name"].as_str() {
                    issue_types.push(IssueType {
                        name: name.to_string(),
                        description: type_obj["description"].as_str().map(|s| s.to_string()),
                        icon_url: type_obj["iconUrl"].as_str().map(|s| s.to_string()),
                        subtask: type_obj["subtask"].as_bool().unwrap_or(false),
                    });
                }
            }
        } else if let Some(type_array) = json.as_array() {
            // Fallback: direct array response
            for type_obj in type_array {
                if let Some(name) = type_obj["name"].as_str() {
                    issue_types.push(IssueType {
                        name: name.to_string(),
                        description: type_obj["description"].as_str().map(|s| s.to_string()),
                        icon_url: type_obj["iconUrl"].as_str().map(|s| s.to_string()),
                        subtask: type_obj["subtask"].as_bool().unwrap_or(false),
                    });
                }
            }
        }

        Ok(issue_types)
    }

    async fn fetch_project_labels(&self, project_key: &str) -> DomainResult<Vec<Label>> {
        let jql = format!("project = {} AND labels is not EMPTY", project_key);

        let response = self
            .http_client
            .get(format!("{}/rest/api/3/search/jql", self.base_url))
            .query(&[
                ("jql", &jql),
                ("fields", &"labels".to_string()),
                ("maxResults", &"1000".to_string()),
            ])
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to fetch labels: {}", e)))?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to parse labels: {}", e)))?;

        let mut label_set = std::collections::HashSet::new();
        if let Some(issues) = json["issues"].as_array() {
            for issue in issues {
                if let Some(labels) = issue["fields"]["labels"].as_array() {
                    for label in labels {
                        if let Some(label_str) = label.as_str() {
                            label_set.insert(label_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(label_set.into_iter().map(|name| Label { name }).collect())
    }

    async fn fetch_project_components(&self, project_key: &str) -> DomainResult<Vec<Component>> {
        let url = format!(
            "{}/rest/api/3/project/{}/components",
            self.base_url, project_key
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch components: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch components: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse components: {}", e))
        })?;

        let mut components = Vec::new();
        if let Some(component_array) = json.as_array() {
            for component_obj in component_array {
                if let Some(name) = component_obj["name"].as_str() {
                    components.push(Component {
                        name: name.to_string(),
                        description: component_obj["description"].as_str().map(|s| s.to_string()),
                        lead: component_obj["lead"]["displayName"]
                            .as_str()
                            .map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(components)
    }

    async fn fetch_project_versions(&self, project_key: &str) -> DomainResult<Vec<FixVersion>> {
        let url = format!(
            "{}/rest/api/3/project/{}/versions",
            self.base_url, project_key
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch versions: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch versions: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse versions: {}", e))
        })?;

        let mut versions = Vec::new();
        if let Some(version_array) = json.as_array() {
            for version_obj in version_array {
                if let Some(name) = version_obj["name"].as_str() {
                    let release_date = version_obj["releaseDate"]
                        .as_str()
                        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());

                    versions.push(FixVersion {
                        name: name.to_string(),
                        description: version_obj["description"].as_str().map(|s| s.to_string()),
                        released: version_obj["released"].as_bool().unwrap_or(false),
                        release_date,
                    });
                }
            }
        }

        Ok(versions)
    }

    async fn fetch_fields(&self) -> DomainResult<Vec<JiraField>> {
        let url = format!("{}/rest/api/3/field", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to fetch fields: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to fetch fields: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse fields response: {}", e))
        })?;

        let mut fields = Vec::new();
        if let Some(field_array) = json.as_array() {
            for field_obj in field_array {
                let id = match field_obj["id"].as_str() {
                    Some(id) => id.to_string(),
                    None => continue,
                };

                let key = field_obj["key"].as_str().unwrap_or(&id).to_string();

                let name = field_obj["name"].as_str().unwrap_or("").to_string();

                let custom = field_obj["custom"].as_bool().unwrap_or(false);
                let searchable = field_obj["searchable"].as_bool().unwrap_or(false);
                let navigable = field_obj["navigable"].as_bool().unwrap_or(false);
                let orderable = field_obj["orderable"].as_bool().unwrap_or(false);

                // Parse schema information
                let schema = &field_obj["schema"];
                let schema_type = schema["type"].as_str().map(|s| s.to_string());
                let schema_items = schema["items"].as_str().map(|s| s.to_string());
                let schema_system = schema["system"].as_str().map(|s| s.to_string());
                let schema_custom = schema["custom"].as_str().map(|s| s.to_string());
                let schema_custom_id = schema["customId"].as_i64();

                fields.push(JiraField {
                    id,
                    key,
                    name,
                    custom,
                    searchable,
                    navigable,
                    orderable,
                    schema_type,
                    schema_items,
                    schema_system,
                    schema_custom,
                    schema_custom_id,
                });
            }
        }

        Ok(fields)
    }

    async fn create_issue(
        &self,
        project_key: &str,
        summary: &str,
        description: Option<&str>,
        issue_type: &str,
    ) -> DomainResult<CreatedIssueDto> {
        let url = format!("{}/rest/api/3/issue", self.base_url);

        let mut fields = serde_json::json!({
            "project": {
                "key": project_key
            },
            "summary": summary,
            "issuetype": {
                "name": issue_type
            }
        });

        if let Some(desc) = description {
            fields["description"] = serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": desc
                            }
                        ]
                    }
                ]
            });
        }

        let body = serde_json::json!({
            "fields": fields
        });

        debug!(
            "[JIRA API] POST {} (creating issue: project={}, type={}, summary={})",
            url, project_key, issue_type, summary
        );

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::ExternalService(format!("Failed to create issue: {}", e)))?;

        debug!("[JIRA API] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to create issue: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse create issue response: {}", e))
        })?;

        let id = json["id"]
            .as_str()
            .ok_or_else(|| DomainError::ExternalService("Response missing 'id' field".to_string()))?
            .to_string();

        let key = json["key"]
            .as_str()
            .ok_or_else(|| {
                DomainError::ExternalService("Response missing 'key' field".to_string())
            })?
            .to_string();

        let self_url = json["self"].as_str().map(|s| s.to_string());

        Ok(CreatedIssueDto::new(id, key, self_url))
    }

    async fn get_issue_transitions(&self, issue_key: &str) -> DomainResult<Vec<TransitionDto>> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            self.base_url, issue_key
        );

        debug!(
            "[JIRA API] GET {} (fetching transitions for {})",
            url, issue_key
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to get transitions: {}", e))
            })?;

        debug!("[JIRA API] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to get transitions: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse transitions response: {}", e))
        })?;

        let mut transitions = Vec::new();
        if let Some(transition_array) = json["transitions"].as_array() {
            for transition_obj in transition_array {
                let id = transition_obj["id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let name = transition_obj["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let to_status = transition_obj["to"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let to_status_category = transition_obj["to"]["statusCategory"]["key"]
                    .as_str()
                    .map(|s| s.to_string());

                transitions.push(TransitionDto::new(id, name, to_status, to_status_category));
            }
        }

        Ok(transitions)
    }

    async fn transition_issue(&self, issue_key: &str, transition_id: &str) -> DomainResult<()> {
        let url = format!(
            "{}/rest/api/3/issue/{}/transitions",
            self.base_url, issue_key
        );

        let body = serde_json::json!({
            "transition": {
                "id": transition_id
            }
        });

        debug!(
            "[JIRA API] POST {} (transitioning {} with id={})",
            url, issue_key, transition_id
        );

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to transition issue: {}", e))
            })?;

        debug!("[JIRA API] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to transition issue: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn get_issue_count_by_status(
        &self,
        project_key: &str,
    ) -> DomainResult<std::collections::HashMap<String, usize>> {
        use log::info;

        // Get all statuses for the project first
        let statuses = self.fetch_project_statuses(project_key).await?;
        info!(
            "[get_issue_count_by_status] Found {} statuses for project {}",
            statuses.len(),
            project_key
        );

        let mut result = std::collections::HashMap::new();

        // For each status, count issues using JQL
        for status in statuses {
            let jql = format!("project = {} AND status = \"{}\"", project_key, status.name);

            let url = format!("{}/rest/api/3/search/jql", self.base_url);

            // Build request body for POST
            let request_body = serde_json::json!({
                "jql": jql,
                "maxResults": 1,  // Minimal results, we just need total
                "fields": ["key"]  // Minimal fields
            });

            // Use POST method for /rest/api/3/search/jql endpoint
            let response = self
                .http_client
                .post(&url)
                .header("Authorization", &self.auth_header)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    DomainError::ExternalService(format!("Failed to count issues: {}", e))
                })?;

            if response.status().is_success() {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    DomainError::ExternalService(format!("Failed to parse count response: {}", e))
                })?;

                // Try to get total from response, fallback to counting issues array
                let count = json["total"]
                    .as_i64()
                    .or_else(|| json["issues"].as_array().map(|arr| arr.len() as i64))
                    .unwrap_or(0) as usize;

                info!(
                    "[get_issue_count_by_status] Status '{}': {} issues",
                    status.name, count
                );

                if count > 0 {
                    result.insert(status.name, count);
                }
            } else {
                info!(
                    "[get_issue_count_by_status] Failed to get count for status '{}': HTTP {}",
                    status.name,
                    response.status()
                );
            }
        }

        info!(
            "[get_issue_count_by_status] Total statuses with issues: {}",
            result.len()
        );
        Ok(result)
    }

    /// Get total issue count for a project using JQL
    /// Uses pagination to count all issues accurately
    async fn get_total_issue_count(&self, project_key: &str) -> DomainResult<usize> {
        use log::info;

        let jql = format!("project = {}", project_key);
        let url = format!("{}/rest/api/3/search/jql", self.base_url);

        info!(
            "[get_total_issue_count] Starting count for project: {}",
            project_key
        );

        // Build request body for POST
        let request_body = serde_json::json!({
            "jql": jql,
            "maxResults": 1,  // Minimal results, we just need total
            "fields": ["key"]  // Minimal fields
        });

        // Use POST method for /rest/api/3/search/jql endpoint
        let response = self
            .http_client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to get issue count: {}", e))
            })?;

        let status = response.status();
        info!("[get_total_issue_count] API response status: {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            info!("[get_total_issue_count] API error body: {}", body);
            return Err(DomainError::ExternalService(format!(
                "Failed to get issue count: HTTP {}",
                status
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse count response: {}", e))
        })?;

        info!(
            "[get_total_issue_count] API response: total={:?}",
            json["total"]
        );

        // Try to get total from response
        // Note: JIRA Cloud may not return accurate total for large datasets
        // In that case, we need to paginate through all results
        if let Some(total) = json["total"].as_i64() {
            info!("[get_total_issue_count] Got total from API: {}", total);
            if total > 0 {
                return Ok(total as usize);
            }
        }

        info!("[get_total_issue_count] API total was 0 or missing, using pagination fallback");

        // Fallback: paginate through all issues to get accurate count
        // This is slower but more reliable
        let mut count = 0usize;
        let mut page_token: Option<String> = None;

        loop {
            let progress = self
                .fetch_project_issues_batch(project_key, None, page_token.as_deref(), 100)
                .await?;

            count += progress.issues.len();
            info!(
                "[get_total_issue_count] Pagination batch: {} issues, total so far: {}",
                progress.issues.len(),
                count
            );

            if !progress.has_more || progress.issues.is_empty() {
                break;
            }

            page_token = progress.next_page_token;
        }

        info!(
            "[get_total_issue_count] Final count via pagination: {}",
            count
        );
        Ok(count)
    }

    async fn create_issue_link(
        &self,
        link_type: &str,
        inward_issue: &str,
        outward_issue: &str,
    ) -> DomainResult<()> {
        let url = format!("{}/rest/api/3/issueLink", self.base_url);

        let body = serde_json::json!({
            "type": {
                "name": link_type
            },
            "inwardIssue": {
                "key": inward_issue
            },
            "outwardIssue": {
                "key": outward_issue
            }
        });

        debug!(
            "[JIRA API] POST {} (creating link: {} -> {} [{}])",
            url, outward_issue, inward_issue, link_type
        );

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create issue link: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to create issue link: {} - {}",
                status, error_text
            )));
        }

        info!(
            "Created issue link: {} -> {} [{}]",
            outward_issue, inward_issue, link_type
        );
        Ok(())
    }

    async fn update_issue_due_date(&self, issue_key: &str, due_date: &str) -> DomainResult<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.base_url, issue_key);

        let body = serde_json::json!({
            "fields": {
                "duedate": due_date
            }
        });

        debug!("[JIRA API] PUT {} (updating due date to {})", url, due_date);

        let response = self
            .http_client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to update issue due date: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "Failed to update issue due date: {} - {}",
                status, error_text
            )));
        }

        info!("Updated due date for {}: {}", issue_key, due_date);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jira_datetime() {
        // JIRA format without colon in timezone
        let result = parse_jira_datetime("2024-01-15T10:30:00.000+0000");
        assert!(result.is_some(), "Should parse JIRA format with +0000");

        let result = parse_jira_datetime("2024-01-15T10:30:00.000+0900");
        assert!(result.is_some(), "Should parse JIRA format with +0900");

        // RFC3339 format (already valid)
        let result = parse_jira_datetime("2024-01-15T10:30:00.000Z");
        assert!(result.is_some(), "Should parse RFC3339 with Z");

        let result = parse_jira_datetime("2024-01-15T10:30:00.000+00:00");
        assert!(result.is_some(), "Should parse RFC3339 with +00:00");

        // Without milliseconds
        let result = parse_jira_datetime("2024-01-15T10:30:00+0000");
        assert!(result.is_some(), "Should parse without milliseconds");
    }
}
