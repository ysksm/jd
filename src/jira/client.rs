use crate::config::JiraConfig;
use crate::error::{JiraDbError, Result};
use crate::jira::models::{Issue, Project};

pub struct JiraClient {
    client: jira_api::JiraClient,
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
        let projects = jira_api::get_projects(&self.client)
            .await
            .map_err(|e| JiraDbError::JiraApi(e.to_string()))?;

        Ok(projects.into_iter().map(|p| p.into()).collect())
    }

    /// Fetch all issues for a project
    pub async fn fetch_project_issues(&self, project_key: &str) -> Result<Vec<Issue>> {
        let jql = format!("project = {} ORDER BY created DESC", project_key);

        let responses = jira_api::search_all_issues_paginated(&self.client, jql, Some(100))
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
        // Try to fetch projects as a connection test
        jira_api::get_projects(&self.client)
            .await
            .map_err(|e| JiraDbError::JiraApi(format!("Connection test failed: {}", e)))?;

        Ok(())
    }
}
