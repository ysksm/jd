//! Report command handlers

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    generate_interactive_report, generate_static_report, DuckDbChangeHistoryRepository,
    DuckDbIssueRepository, GenerateReportUseCase,
};

use crate::generated::*;
use crate::state::AppState;

/// Generate HTML report
#[tauri::command]
pub async fn reports_generate(
    state: State<'_, AppState>,
    request: ReportGenerateRequest,
) -> Result<ReportGenerateResponse, String> {
    let settings = state.get_settings().ok_or("Not initialized")?;
    let db = state.get_db().ok_or("Database not initialized")?;

    // Get projects to include in report
    let projects_to_report: Vec<_> = if let Some(ref project_key) = request.project_key {
        settings
            .projects
            .iter()
            .filter(|p| &p.key == project_key)
            .collect()
    } else {
        settings
            .projects
            .iter()
            .filter(|p| p.sync_enabled)
            .collect()
    };

    if projects_to_report.is_empty() {
        return Err("No projects to include in report".to_string());
    }

    // Create repositories
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db));

    // Create use case
    let use_case = GenerateReportUseCase::new(issue_repo, change_history_repo);

    // Build project keys
    let project_keys: Vec<(&str, &str, &str)> = projects_to_report
        .iter()
        .map(|p| (p.id.as_str(), p.key.as_str(), p.name.as_str()))
        .collect();

    // Generate report data
    let report_data = use_case.execute(&project_keys).map_err(|e| e.to_string())?;

    // Determine output path
    let output_path = request
        .output_path
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let suffix = if request.interactive.unwrap_or(false) {
                "interactive"
            } else {
                "static"
            };
            PathBuf::from(format!("jira_report_{}_{}.html", suffix, timestamp))
        });

    // Generate HTML - these functions return String directly, not Result
    let html_content = if request.interactive.unwrap_or(false) {
        generate_interactive_report(&report_data)
    } else {
        generate_static_report(&report_data)
    };

    // Write to file
    std::fs::write(&output_path, html_content).map_err(|e| e.to_string())?;

    Ok(ReportGenerateResponse {
        result: ReportResult {
            output_path: output_path.to_string_lossy().to_string(),
            issue_count: report_data.total_issues as i32,
        },
    })
}
