//! Report command handlers

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use jira_db_core::{
    DuckDbChangeHistoryRepository, DuckDbIssueRepository, GenerateReportUseCase,
    generate_interactive_report, generate_static_report,
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

    // Project key is required for per-project database
    let project_key = request
        .project_key
        .as_ref()
        .ok_or("project_key is required for report generation (per-project database)")?;

    let db = state
        .get_db(project_key)
        .ok_or_else(|| format!("Database not initialized for project {}", project_key))?;

    // Get the project to include in report
    let project = settings
        .projects
        .iter()
        .find(|p| &p.key == project_key)
        .ok_or_else(|| format!("Project {} not found", project_key))?;

    // Create repositories
    let issue_repo = Arc::new(DuckDbIssueRepository::new(db.clone()));
    let change_history_repo = Arc::new(DuckDbChangeHistoryRepository::new(db));

    // Create use case
    let use_case = GenerateReportUseCase::new(issue_repo, change_history_repo);

    // Build project keys (single project)
    let project_keys: Vec<(&str, &str, &str)> = vec![(
        project.id.as_str(),
        project.key.as_str(),
        project.name.as_str(),
    )];

    // Generate report data
    let report_data = use_case.execute(&project_keys).map_err(|e| e.to_string())?;

    // Determine output path
    let output_path = request.output_path.map(PathBuf::from).unwrap_or_else(|| {
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
