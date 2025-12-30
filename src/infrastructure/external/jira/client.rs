use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use log::warn;
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::application::dto::CreatedIssueDto;
use crate::application::services::JiraService;
use crate::domain::entities::{
    Component, FixVersion, Issue, IssueType, Label, Priority, Project, Status,
};
use crate::domain::error::{DomainError, DomainResult};
use crate::infrastructure::config::JiraConfig;

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
}

#[async_trait]
impl JiraService for JiraApiClient {
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>> {
        let client = &self.client;

        let projects = retry_with_backoff(
            || async { jira_api::get_projects(client).await },
            3,
            30,
        )
        .await
        .map_err(|e| DomainError::ExternalService(e.to_string()))?;

        Ok(projects
            .into_iter()
            .map(|p| Project::new(p.id, p.key, p.name, p.description))
            .collect())
    }

    async fn fetch_project_issues(&self, project_key: &str) -> DomainResult<Vec<Issue>> {
        let jql = format!("project = {} ORDER BY created DESC", project_key);
        let url = format!("{}/rest/api/3/search/jql", self.base_url);

        let mut all_issues = Vec::new();
        let mut start_at = 0;
        let max_results = 100;

        loop {
            let response = self
                .http_client
                .get(&url)
                .query(&[
                    ("jql", jql.as_str()),
                    ("fields", "*navigable"),
                    ("expand", "changelog"),
                    ("maxResults", &max_results.to_string()),
                    ("startAt", &start_at.to_string()),
                ])
                .header("Authorization", &self.auth_header)
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| {
                    DomainError::ExternalService(format!("Failed to fetch issues: {}", e))
                })?;

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

            let json: serde_json::Value = response.json().await.map_err(|e| {
                DomainError::ExternalService(format!("Failed to parse issues: {}", e))
            })?;

            let total = json["total"].as_i64().unwrap_or(0);

            if let Some(issues_array) = json["issues"].as_array() {
                for issue_json in issues_array {
                    if let (Some(id), Some(key)) =
                        (issue_json["id"].as_str(), issue_json["key"].as_str())
                    {
                        let fields = &issue_json["fields"];

                        let project_id = fields["project"]["id"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        let summary = fields["summary"].as_str().unwrap_or("").to_string();

                        let description = fields["description"].as_str().map(|s| s.to_string());

                        let status = fields["status"]["name"].as_str().map(|s| s.to_string());

                        let priority = fields["priority"]["name"].as_str().map(|s| s.to_string());

                        let assignee =
                            fields["assignee"]["displayName"].as_str().map(|s| s.to_string());

                        let reporter =
                            fields["reporter"]["displayName"].as_str().map(|s| s.to_string());

                        let issue_type =
                            fields["issuetype"]["name"].as_str().map(|s| s.to_string());

                        let resolution =
                            fields["resolution"]["name"].as_str().map(|s| s.to_string());

                        let labels = fields["labels"].as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        }).filter(|v| !v.is_empty());

                        let components = fields["components"].as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        }).filter(|v| !v.is_empty());

                        let fix_versions = fields["fixVersions"].as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        }).filter(|v| !v.is_empty());

                        let parent_key = fields["parent"]["key"].as_str().map(|s| s.to_string());

                        let created_date = fields["created"]
                            .as_str()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        let updated_date = fields["updated"]
                            .as_str()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        let raw_json = serde_json::to_string(&issue_json).ok();

                        all_issues.push(Issue::new(
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
                            parent_key,
                            created_date,
                            updated_date,
                            raw_json,
                        ));
                    }
                }

                if (start_at + max_results) >= total as usize {
                    break;
                }
                start_at += max_results;
            } else {
                break;
            }
        }

        Ok(all_issues)
    }

    async fn test_connection(&self) -> DomainResult<()> {
        let client = &self.client;

        retry_with_backoff(
            || async { jira_api::get_projects(client).await },
            2,
            15,
        )
        .await
        .map_err(|e| DomainError::ExternalService(format!("Connection test failed: {}", e)))?;

        Ok(())
    }

    async fn fetch_project_statuses(&self, project_key: &str) -> DomainResult<Vec<Status>> {
        let url = format!(
            "{}/rest/api/3/project/{}/statuses",
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
                DomainError::ExternalService(format!("Failed to fetch statuses: {}", e))
            })?;

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
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to fetch labels: {}", e))
            })?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse labels: {}", e))
        })?;

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
                DomainError::ExternalService(format!("Failed to create issue: {}", e))
            })?;

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
}
