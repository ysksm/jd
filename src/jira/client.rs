use crate::config::JiraConfig;
use crate::error::{JiraDbError, Result};
use crate::jira::models::{Component, FixVersion, Issue, IssueType, Label, Priority, Project, Status};
use base64::{engine::general_purpose, Engine as _};
use log::warn;
use std::time::Duration;
use tokio::time::{sleep, timeout};

pub struct JiraClient {
    client: jira_api::JiraClient,
    http_client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

/// Retry a future with exponential backoff
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
        // Wrap the operation in a timeout
        let result = match timeout(timeout_duration, f()).await {
            Ok(result) => result,
            Err(_) => {
                warn!("Operation timed out after {} seconds", timeout_secs);
                if retry_count >= max_retries {
                    return Err(f().await.unwrap_err());
                }
                retry_count += 1;
                let delay = Duration::from_secs(2u64.pow(retry_count));
                warn!("Retrying in {:?} (attempt {}/{})", delay, retry_count, max_retries);
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
                warn!("Request failed: {}. Retrying in {:?} (attempt {}/{})", e, delay, retry_count, max_retries);
                sleep(delay).await;
            }
        }
    }
}

impl JiraClient {
    /// Create a new JIRA client
    pub fn new(config: &JiraConfig) -> Result<Self> {
        let client = jira_api::JiraClient::new(
            config.endpoint.clone(),
            config.username.clone(),
            config.api_key.clone(),
        );

        // Create HTTP client for direct API calls
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to create HTTP client: {}", e)))?;

        // Create Basic Auth header
        let credentials = format!("{}:{}", config.username, config.api_key);
        let auth_header = format!("Basic {}", general_purpose::STANDARD.encode(credentials));

        Ok(Self {
            client,
            http_client,
            base_url: config.endpoint.clone(),
            auth_header,
        })
    }

    /// Fetch all projects from JIRA
    pub async fn fetch_projects(&self) -> Result<Vec<Project>> {
        let client = &self.client;

        let projects = retry_with_backoff(
            || async { jira_api::get_projects(client).await },
            3, // max 3 retries
            30, // 30 second timeout
        )
        .await
        .map_err(|e| JiraDbError::JiraApi(e.to_string()))?;

        Ok(projects.into_iter().map(|p| p.into()).collect())
    }

    /// Fetch all issues for a project
    pub async fn fetch_project_issues(&self, project_key: &str) -> Result<Vec<Issue>> {
        let jql = format!("project = {} ORDER BY created DESC", project_key);
        let client = &self.client;
        let jql_clone = jql.clone();

        let responses = retry_with_backoff(
            || async { jira_api::search_all_issues_paginated(client, jql_clone.clone(), Some(100)).await },
            3, // max 3 retries
            60, // 60 second timeout (longer for potentially large result sets)
        )
        .await
        .map_err(|e| JiraDbError::JiraApi(e.to_string()))?;

        // Flatten all issues from all pages
        let issues: Vec<Issue> = responses
            .into_iter()
            .flat_map(|response| response.issues)
            .map(|i| i.into())
            .collect();

        Ok(issues)
    }

    /// Test connection to JIRA
    pub async fn test_connection(&self) -> Result<()> {
        let client = &self.client;

        // Try to fetch projects as a connection test with retry
        retry_with_backoff(
            || async { jira_api::get_projects(client).await },
            2, // max 2 retries for connection test
            15, // 15 second timeout
        )
        .await
        .map_err(|e| JiraDbError::JiraApi(format!("Connection test failed: {}", e)))?;

        Ok(())
    }

    /// Fetch all statuses for a project from JIRA
    pub async fn fetch_project_statuses(&self, project_key: &str) -> Result<Vec<Status>> {
        let url = format!("{}/rest/api/3/project/{}/statuses", self.base_url, project_key);

        let response = self.http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch statuses: {}", e)))?;

        if !response.status().is_success() {
            return Err(JiraDbError::JiraApi(format!("Failed to fetch statuses: {}", response.status())));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse statuses: {}", e)))?;

        let mut statuses = Vec::new();
        if let Some(issue_types) = json.as_array() {
            for issue_type in issue_types {
                if let Some(status_array) = issue_type["statuses"].as_array() {
                    for status_obj in status_array {
                        if let Some(name) = status_obj["name"].as_str() {
                            statuses.push(Status {
                                name: name.to_string(),
                                description: status_obj["description"].as_str().map(|s| s.to_string()),
                                category: status_obj["statusCategory"]["key"].as_str().map(|s| s.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(statuses)
    }

    /// Fetch all priorities from JIRA
    pub async fn fetch_priorities(&self) -> Result<Vec<Priority>> {
        let url = format!("{}/rest/api/3/priority", self.base_url);

        let response = self.http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch priorities: {}", e)))?;

        if !response.status().is_success() {
            return Err(JiraDbError::JiraApi(format!("Failed to fetch priorities: {}", response.status())));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse priorities: {}", e)))?;

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

    /// Fetch all issue types for a project from JIRA
    pub async fn fetch_project_issue_types(&self, project_id: &str) -> Result<Vec<IssueType>> {
        let url = format!("{}/rest/api/3/issuetype/project?projectId={}", self.base_url, project_id);

        let response = self.http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch issue types: {}", e)))?;

        if !response.status().is_success() {
            return Err(JiraDbError::JiraApi(format!("Failed to fetch issue types: {}", response.status())));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse issue types: {}", e)))?;

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

    /// Fetch all labels for a project (from issues)
    pub async fn fetch_project_labels(&self, project_key: &str) -> Result<Vec<Label>> {
        // JIRA doesn't have a direct API for all labels in a project
        // We'll get them from the issues we've already fetched
        let jql = format!("project = {} AND labels is not EMPTY", project_key);

        let response = self.http_client
            .get(format!("{}/rest/api/3/search", self.base_url))
            .query(&[("jql", &jql), ("fields", &"labels".to_string()), ("maxResults", &"1000".to_string())])
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch labels: {}", e)))?;

        if !response.status().is_success() {
            // If there are no issues with labels, return empty vector
            return Ok(Vec::new());
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse labels: {}", e)))?;

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

    /// Fetch all components for a project from JIRA
    pub async fn fetch_project_components(&self, project_key: &str) -> Result<Vec<Component>> {
        let url = format!("{}/rest/api/3/project/{}/components", self.base_url, project_key);

        let response = self.http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch components: {}", e)))?;

        if !response.status().is_success() {
            return Err(JiraDbError::JiraApi(format!("Failed to fetch components: {}", response.status())));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse components: {}", e)))?;

        let mut components = Vec::new();
        if let Some(component_array) = json.as_array() {
            for component_obj in component_array {
                if let Some(name) = component_obj["name"].as_str() {
                    components.push(Component {
                        name: name.to_string(),
                        description: component_obj["description"].as_str().map(|s| s.to_string()),
                        lead: component_obj["lead"]["displayName"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(components)
    }

    /// Fetch all fix versions for a project from JIRA
    pub async fn fetch_project_versions(&self, project_key: &str) -> Result<Vec<FixVersion>> {
        let url = format!("{}/rest/api/3/project/{}/versions", self.base_url, project_key);

        let response = self.http_client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to fetch versions: {}", e)))?;

        if !response.status().is_success() {
            return Err(JiraDbError::JiraApi(format!("Failed to fetch versions: {}", response.status())));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| JiraDbError::JiraApi(format!("Failed to parse versions: {}", e)))?;

        let mut versions = Vec::new();
        if let Some(version_array) = json.as_array() {
            for version_obj in version_array {
                if let Some(name) = version_obj["name"].as_str() {
                    let release_date = version_obj["releaseDate"].as_str()
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
}
