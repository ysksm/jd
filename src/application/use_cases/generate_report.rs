use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
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
    pub sprint_counts: HashMap<String, usize>,
    pub timeline_data: Vec<TimelineDataPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineDataPoint {
    pub date: String,
    pub created: usize,
    pub resolved: usize,
    pub active: usize,
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
    pub sprint: String,
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
        let mut sprint_counts: HashMap<String, usize> = HashMap::new();

        let mut issue_data_list = Vec::new();

        for issue in issues {
            let status = issue.status.clone().unwrap_or_else(|| "Unknown".to_string());
            let priority = issue.priority.clone().unwrap_or_else(|| "Unknown".to_string());
            let assignee = issue.assignee.clone().unwrap_or_else(|| "Unassigned".to_string());
            let issue_type = issue.issue_type.clone().unwrap_or_else(|| "Unknown".to_string());
            let sprint = issue.sprint.clone().unwrap_or_else(|| "No Sprint".to_string());
            let components = issue.components.clone().unwrap_or_default();
            let labels = issue.labels.clone().unwrap_or_default();
            let reporter = issue.reporter.clone().unwrap_or_else(|| "Unknown".to_string());

            *status_counts.entry(status.clone()).or_insert(0) += 1;
            *priority_counts.entry(priority.clone()).or_insert(0) += 1;
            *assignee_counts.entry(assignee.clone()).or_insert(0) += 1;
            *issue_type_counts.entry(issue_type.clone()).or_insert(0) += 1;
            *sprint_counts.entry(sprint.clone()).or_insert(0) += 1;

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
                sprint,
                components,
                labels,
                created_date: issue.created_date,
                updated_date: issue.updated_date,
                change_history: history_data,
            });
        }

        // Calculate timeline data for burndown chart
        let timeline_data = self.calculate_timeline_data(&issue_data_list);

        Ok(ProjectReportData {
            key: project_key.to_string(),
            name: project_name.to_string(),
            issues: issue_data_list,
            status_counts,
            priority_counts,
            assignee_counts,
            issue_type_counts,
            component_counts,
            sprint_counts,
            timeline_data,
        })
    }

    fn calculate_timeline_data(&self, issues: &[IssueReportData]) -> Vec<TimelineDataPoint> {
        // Collect all events (created and resolved)
        let mut created_by_date: BTreeMap<NaiveDate, usize> = BTreeMap::new();
        let mut resolved_by_date: BTreeMap<NaiveDate, usize> = BTreeMap::new();

        for issue in issues {
            // Track creation date
            if let Some(created) = issue.created_date {
                let date = created.date_naive();
                *created_by_date.entry(date).or_insert(0) += 1;
            }

            // Track resolution date from change history
            for change in &issue.change_history {
                if change.field.to_lowercase() == "status" {
                    let to_status = change.to_string.to_lowercase();
                    if to_status.contains("done")
                        || to_status.contains("closed")
                        || to_status.contains("resolved")
                        || to_status.contains("complete") {
                        let date = change.changed_at.date_naive();
                        *resolved_by_date.entry(date).or_insert(0) += 1;
                        break; // Only count first resolution
                    }
                }
            }
        }

        // If no dates found, return empty
        if created_by_date.is_empty() {
            return Vec::new();
        }

        // Get date range
        let min_date = *created_by_date.keys().next().unwrap();
        let max_date = Utc::now().date_naive();

        // Build cumulative timeline
        let mut timeline = Vec::new();
        let mut cumulative_created = 0usize;
        let mut cumulative_resolved = 0usize;
        let mut current_date = min_date;

        while current_date <= max_date {
            let created_today = created_by_date.get(&current_date).copied().unwrap_or(0);
            let resolved_today = resolved_by_date.get(&current_date).copied().unwrap_or(0);

            cumulative_created += created_today;
            cumulative_resolved += resolved_today;

            let active = cumulative_created.saturating_sub(cumulative_resolved);

            timeline.push(TimelineDataPoint {
                date: current_date.format("%Y-%m-%d").to_string(),
                created: cumulative_created,
                resolved: cumulative_resolved,
                active,
            });

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        // Reduce data points if too many (keep weekly for long periods)
        if timeline.len() > 90 {
            let step = timeline.len() / 60;
            timeline = timeline.into_iter()
                .enumerate()
                .filter(|(i, _)| i % step == 0 || *i == 0)
                .map(|(_, point)| point)
                .collect();
        }

        timeline
    }
}
