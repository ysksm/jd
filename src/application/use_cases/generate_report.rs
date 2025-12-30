use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::entities::Issue;
use crate::domain::error::DomainResult;
use crate::domain::repositories::{ChangeHistoryRepository, IssueRepository};

#[derive(Debug, Clone, Serialize)]
pub struct ReportData {
    pub generated_at: DateTime<Utc>,
    pub projects: Vec<ProjectReportData>,
    pub total_issues: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectReportData {
    pub key: String,
    pub name: String,
    pub issues: Vec<IssueReportData>,
    pub status_counts: HashMap<String, usize>,
    pub priority_counts: HashMap<String, usize>,
    pub assignee_counts: HashMap<String, usize>,
    pub issue_type_counts: HashMap<String, usize>,
    pub component_counts: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueReportData {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub reporter: String,
    pub issue_type: String,
    pub components: Vec<String>,
    pub labels: Vec<String>,
    pub created_date: Option<DateTime<Utc>>,
    pub updated_date: Option<DateTime<Utc>>,
    pub change_history: Vec<ChangeHistoryData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeHistoryData {
    pub field: String,
    pub from_string: String,
    pub to_string: String,
    pub changed_at: DateTime<Utc>,
    pub author: String,
}

pub struct GenerateReportUseCase<I, C>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
{
    issue_repository: Arc<I>,
    change_history_repository: Arc<C>,
}

impl<I, C> GenerateReportUseCase<I, C>
where
    I: IssueRepository,
    C: ChangeHistoryRepository,
{
    pub fn new(issue_repository: Arc<I>, change_history_repository: Arc<C>) -> Self {
        Self {
            issue_repository,
            change_history_repository,
        }
    }

    pub fn execute(&self, project_keys: &[(&str, &str, &str)]) -> DomainResult<ReportData> {
        let mut projects = Vec::new();
        let mut total_issues = 0;

        for (project_id, project_key, project_name) in project_keys {
            let issues = self.issue_repository.find_by_project(project_id)?;
            total_issues += issues.len();

            let project_data = self.build_project_data(project_key, project_name, &issues)?;
            projects.push(project_data);
        }

        Ok(ReportData {
            generated_at: Utc::now(),
            projects,
            total_issues,
        })
    }

    fn build_project_data(
        &self,
        project_key: &str,
        project_name: &str,
        issues: &[Issue],
    ) -> DomainResult<ProjectReportData> {
        let mut status_counts: HashMap<String, usize> = HashMap::new();
        let mut priority_counts: HashMap<String, usize> = HashMap::new();
        let mut assignee_counts: HashMap<String, usize> = HashMap::new();
        let mut issue_type_counts: HashMap<String, usize> = HashMap::new();
        let mut component_counts: HashMap<String, usize> = HashMap::new();

        let mut issue_data_list = Vec::new();

        for issue in issues {
            let status = issue.status.clone().unwrap_or_else(|| "Unknown".to_string());
            let priority = issue.priority.clone().unwrap_or_else(|| "Unknown".to_string());
            let assignee = issue.assignee.clone().unwrap_or_else(|| "Unassigned".to_string());
            let issue_type = issue.issue_type.clone().unwrap_or_else(|| "Unknown".to_string());
            let components = issue.components.clone().unwrap_or_default();
            let labels = issue.labels.clone().unwrap_or_default();
            let reporter = issue.reporter.clone().unwrap_or_else(|| "Unknown".to_string());

            *status_counts.entry(status.clone()).or_insert(0) += 1;
            *priority_counts.entry(priority.clone()).or_insert(0) += 1;
            *assignee_counts.entry(assignee.clone()).or_insert(0) += 1;
            *issue_type_counts.entry(issue_type.clone()).or_insert(0) += 1;

            for component in &components {
                *component_counts.entry(component.clone()).or_insert(0) += 1;
            }

            let change_history = self
                .change_history_repository
                .find_by_issue_key(&issue.key)?;

            let history_data: Vec<ChangeHistoryData> = change_history
                .into_iter()
                .map(|h| ChangeHistoryData {
                    field: h.field,
                    from_string: h.from_string.unwrap_or_else(|| "-".to_string()),
                    to_string: h.to_string.unwrap_or_else(|| "-".to_string()),
                    changed_at: h.changed_at,
                    author: h.author_display_name.unwrap_or_else(|| "Unknown".to_string()),
                })
                .collect();

            issue_data_list.push(IssueReportData {
                key: issue.key.clone(),
                summary: issue.summary.clone(),
                status,
                priority,
                assignee,
                reporter,
                issue_type,
                components,
                labels,
                created_date: issue.created_date,
                updated_date: issue.updated_date,
                change_history: history_data,
            });
        }

        Ok(ProjectReportData {
            key: project_key.to_string(),
            name: project_name.to_string(),
            issues: issue_data_list,
            status_counts,
            priority_counts,
            assignee_counts,
            issue_type_counts,
            component_counts,
        })
    }
}
