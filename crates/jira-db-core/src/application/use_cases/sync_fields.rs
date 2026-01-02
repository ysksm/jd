use crate::application::services::JiraService;
use crate::domain::entities::JiraField;
use crate::domain::error::DomainResult;
use crate::infrastructure::database::{DuckDbFieldRepository, DuckDbIssuesExpandedRepository};
use std::sync::Arc;

/// Result of field synchronization
#[derive(Debug)]
pub struct SyncFieldsResult {
    /// Number of fields synced
    pub fields_synced: usize,
    /// Number of columns added to issues_expanded
    pub columns_added: usize,
    /// Number of issues expanded
    pub issues_expanded: usize,
}

/// Use case for synchronizing JIRA fields and expanding issues
pub struct SyncFieldsUseCase {
    jira_service: Arc<dyn JiraService>,
    field_repo: Arc<DuckDbFieldRepository>,
    expanded_repo: Arc<DuckDbIssuesExpandedRepository>,
}

impl SyncFieldsUseCase {
    pub fn new(
        jira_service: Arc<dyn JiraService>,
        field_repo: Arc<DuckDbFieldRepository>,
        expanded_repo: Arc<DuckDbIssuesExpandedRepository>,
    ) -> Self {
        Self {
            jira_service,
            field_repo,
            expanded_repo,
        }
    }

    /// Sync fields from JIRA API and store in database
    pub async fn sync_fields(&self) -> DomainResult<usize> {
        let fields = self.jira_service.fetch_fields().await?;
        self.field_repo.upsert_fields(&fields)
    }

    /// Add columns to issues_expanded table based on stored field definitions
    pub fn add_columns(&self) -> DomainResult<Vec<String>> {
        let fields = self.field_repo.find_all()?;
        self.expanded_repo.add_field_columns(&fields)
    }

    /// Expand issues from raw_data into issues_expanded table
    pub fn expand_issues(&self, project_id: Option<&str>) -> DomainResult<usize> {
        self.expanded_repo.expand_issues(project_id)
    }

    /// Execute full sync: fetch fields, add columns, and expand issues
    pub async fn execute(&self, project_id: Option<&str>) -> DomainResult<SyncFieldsResult> {
        // Step 1: Fetch fields from JIRA
        let fields_synced = self.sync_fields().await?;

        // Step 2: Add columns based on fields
        let added_columns = self.add_columns()?;
        let columns_added = added_columns.len();

        // Step 3: Expand issues
        let issues_expanded = self.expand_issues(project_id)?;

        Ok(SyncFieldsResult {
            fields_synced,
            columns_added,
            issues_expanded,
        })
    }

    /// Get all stored fields
    pub fn get_fields(&self) -> DomainResult<Vec<JiraField>> {
        self.field_repo.find_all()
    }

    /// Get navigable fields only
    pub fn get_navigable_fields(&self) -> DomainResult<Vec<JiraField>> {
        self.field_repo.find_navigable()
    }

    /// Get count of expanded issues
    pub fn get_expanded_count(&self, project_id: Option<&str>) -> DomainResult<i64> {
        self.expanded_repo.count(project_id)
    }
}
