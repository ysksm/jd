use crate::config::JiraConfig;
use crate::error::{JiraDbError, Result};
use crate::jira::models::{Issue, Project};
use log::warn;
use std::time::Duration;
use tokio::time::{sleep, timeout};

pub struct JiraClient {
    client: jira_api::JiraClient,
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

        Ok(Self { client })
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
}
