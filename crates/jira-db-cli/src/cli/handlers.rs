#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use dialoguer::{Confirm, Input};
use log::{info, warn};

use jira_db_core::application::services::JiraService;
use jira_db_core::application::use_cases::{
    CreateTestTicketUseCase, GenerateReportUseCase, GenerateSnapshotsUseCase,
    GetChangeHistoryUseCase, GetProjectMetadataUseCase, SearchIssuesUseCase,
    SyncProjectListUseCase, SyncProjectUseCase,
};
use jira_db_core::chrono::Utc;
use jira_db_core::domain::error::{DomainError, DomainResult};
use jira_db_core::domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository, MetadataRepository,
    ProjectRepository, SearchParams, SyncHistoryRepository,
};
use jira_db_core::indicatif::{ProgressBar, ProgressStyle};
use jira_db_core::infrastructure::config::{DatabaseConfig, JiraConfig, ProjectConfig, Settings};
use jira_db_core::report::{generate_interactive_report, generate_static_report};

pub struct CliHandler<P, I, M, C, S, N, J>
where
    P: ProjectRepository,
    I: IssueRepository,
    M: MetadataRepository,
    C: ChangeHistoryRepository,
    S: SyncHistoryRepository,
    N: IssueSnapshotRepository,
    J: JiraService,
{
    project_repository: Arc<P>,
    issue_repository: Arc<I>,
    metadata_repository: Arc<M>,
    change_history_repository: Arc<C>,
    sync_history_repository: Arc<S>,
    snapshot_repository: Arc<N>,
    jira_service: Arc<J>,
    settings_path: PathBuf,
}

impl<P, I, M, C, S, N, J> CliHandler<P, I, M, C, S, N, J>
where
    P: ProjectRepository,
    I: IssueRepository,
    M: MetadataRepository,
    C: ChangeHistoryRepository,
    S: SyncHistoryRepository,
    N: IssueSnapshotRepository,
    J: JiraService,
{
    pub fn new(
        project_repository: Arc<P>,
        issue_repository: Arc<I>,
        metadata_repository: Arc<M>,
        change_history_repository: Arc<C>,
        sync_history_repository: Arc<S>,
        snapshot_repository: Arc<N>,
        jira_service: Arc<J>,
        settings_path: PathBuf,
    ) -> Self {
        Self {
            project_repository,
            issue_repository,
            metadata_repository,
            change_history_repository,
            sync_history_repository,
            snapshot_repository,
            jira_service,
            settings_path,
        }
    }

    pub async fn handle_init(&self, interactive: bool) -> DomainResult<()> {
        if Settings::exists(&self.settings_path) {
            println!(
                "Configuration file already exists at {}",
                self.settings_path.display()
            );
            return Ok(());
        }

        if interactive {
            self.handle_init_interactive().await?;
        } else {
            Settings::create_default(&self.settings_path)?;
            println!(
                "Created default configuration file at {}",
                self.settings_path.display()
            );
            println!("Please edit the file to configure your JIRA connection.");
        }

        Ok(())
    }

    async fn handle_init_interactive(&self) -> DomainResult<()> {
        println!("JIRA-DB Configuration Setup\n");

        let endpoint: String = Input::new()
            .with_prompt("JIRA endpoint (e.g., https://your-domain.atlassian.net)")
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

        let username: String = Input::new()
            .with_prompt("JIRA username (email)")
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

        let api_key: String = Input::new()
            .with_prompt("JIRA API key")
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

        let db_path: String = Input::new()
            .with_prompt("Database path")
            .default("./data/jira.duckdb".into())
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

        let settings = Settings {
            jira: JiraConfig {
                endpoint,
                username,
                api_key,
            },
            projects: Vec::new(),
            database: DatabaseConfig {
                path: PathBuf::from(db_path),
            },
            embeddings: None,
        };

        println!("\nTesting JIRA connection...");
        if let Err(e) = self.jira_service.test_connection().await {
            println!("Warning: Could not connect to JIRA: {}", e);
            let proceed = Confirm::new()
                .with_prompt("Save configuration anyway?")
                .default(true)
                .interact()
                .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

            if !proceed {
                println!("Configuration cancelled.");
                return Ok(());
            }
        } else {
            println!("JIRA connection successful!");
        }

        settings.save(&self.settings_path)?;

        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.settings_path)
                .map_err(|e| DomainError::Repository(format!("Failed to get metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.settings_path, perms).map_err(|e| {
                DomainError::Repository(format!("Failed to set permissions: {}", e))
            })?;
        }

        println!("\nConfiguration saved to {}", self.settings_path.display());

        Ok(())
    }

    pub async fn handle_project_init(&self) -> DomainResult<()> {
        let mut settings = Settings::load(&self.settings_path)?;
        settings.validate()?;

        let use_case =
            SyncProjectListUseCase::new(self.project_repository.clone(), self.jira_service.clone());

        let projects = use_case.execute().await?;

        for project in &projects {
            let project_config = ProjectConfig {
                id: project.id.clone(),
                key: project.key.clone(),
                name: project.name.clone(),
                sync_enabled: false,
                last_synced: None,
                sync_checkpoint: None,
            };
            settings.upsert_project(project_config);
        }

        settings.save(&self.settings_path)?;

        println!("Fetched {} projects from JIRA", projects.len());
        println!("Use 'jira-db project enable <PROJECT_KEY>' to enable sync for a project");

        Ok(())
    }

    pub fn handle_project_list(&self, verbose: bool) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;

        if settings.projects.is_empty() {
            println!("No projects found. Run 'jira-db project init' first.");
            return Ok(());
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);

        if verbose {
            table.set_header(vec!["Key", "Name", "Sync", "ID", "Last Synced"]);

            for project in &settings.projects {
                let last_synced = project
                    .last_synced
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "-".to_string());

                table.add_row(vec![
                    Cell::new(&project.key),
                    Cell::new(&project.name),
                    if project.sync_enabled {
                        Cell::new("✓").fg(Color::Green)
                    } else {
                        Cell::new("✗").fg(Color::Red)
                    },
                    Cell::new(&project.id),
                    Cell::new(last_synced),
                ]);
            }
        } else {
            table.set_header(vec!["Key", "Name", "Sync Enabled"]);

            for project in &settings.projects {
                table.add_row(vec![
                    Cell::new(&project.key),
                    Cell::new(&project.name),
                    if project.sync_enabled {
                        Cell::new("✓").fg(Color::Green)
                    } else {
                        Cell::new("✗").fg(Color::Red)
                    },
                ]);
            }
        }

        println!("{table}");
        Ok(())
    }

    pub fn handle_project_enable(&self, project_key: &str) -> DomainResult<()> {
        let mut settings = Settings::load(&self.settings_path)?;

        if let Some(project) = settings.find_project_mut(project_key) {
            project.sync_enabled = true;
            settings.save(&self.settings_path)?;
            println!("Enabled sync for project: {}", project_key);
        } else {
            return Err(DomainError::NotFound(format!(
                "Project not found: {}",
                project_key
            )));
        }

        Ok(())
    }

    pub fn handle_project_disable(&self, project_key: &str) -> DomainResult<()> {
        let mut settings = Settings::load(&self.settings_path)?;

        if let Some(project) = settings.find_project_mut(project_key) {
            project.sync_enabled = false;
            settings.save(&self.settings_path)?;
            println!("Disabled sync for project: {}", project_key);
        } else {
            return Err(DomainError::NotFound(format!(
                "Project not found: {}",
                project_key
            )));
        }

        Ok(())
    }

    pub async fn handle_sync(&self, project_key: Option<String>) -> DomainResult<()> {
        let mut settings = Settings::load(&self.settings_path)?;
        settings.validate()?;

        if settings.projects.is_empty() {
            return Err(DomainError::Validation(
                "No projects found. Run 'jira-db project init' first.".into(),
            ));
        }

        let use_case = SyncProjectUseCase::new(
            self.issue_repository.clone(),
            self.change_history_repository.clone(),
            self.metadata_repository.clone(),
            self.sync_history_repository.clone(),
            self.snapshot_repository.clone(),
            self.jira_service.clone(),
        );

        let settings_path = self.settings_path.clone();

        if let Some(key) = project_key {
            let project = settings
                .find_project(&key)
                .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", key)))?;

            let project_id = project.id.clone();
            let checkpoint = project.sync_checkpoint.clone();

            // Show resuming message if we have a checkpoint
            if let Some(ref cp) = checkpoint {
                println!(
                    "Resuming sync for {} from checkpoint ({}/{} issues processed)",
                    key, cp.items_processed, cp.total_items
                );
            }

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("[{elapsed_precise}] {spinner:.cyan} {msg}")
                    .expect("Failed to create progress style"),
            );
            pb.set_message(format!("Syncing project {}...", key));

            // Use resumable sync with checkpoint saving callback
            let result = use_case
                .execute_resumable(&key, &project_id, checkpoint, |new_checkpoint| {
                    // Update progress bar with checkpoint info
                    pb.set_message(format!(
                        "Syncing project {}... ({}/{} issues)",
                        key, new_checkpoint.items_processed, new_checkpoint.total_items
                    ));

                    // Save checkpoint to settings
                    if let Ok(mut s) = Settings::load(&settings_path) {
                        if let Some(p) = s.find_project_mut(&key) {
                            p.sync_checkpoint = Some(new_checkpoint.clone());
                        }
                        let _ = s.save(&settings_path);
                    }
                })
                .await?;

            pb.finish_and_clear();

            if result.sync_result.success {
                println!(
                    "Synced {} issues ({} history items) for project {}",
                    result.sync_result.issues_synced, result.sync_result.history_items_synced, key
                );

                // Clear checkpoint on success
                if let Some(p) = settings.find_project_mut(&key) {
                    p.last_synced = Some(Utc::now());
                    p.sync_checkpoint = None;
                }
                settings.save(&self.settings_path)?;
            } else {
                println!(
                    "Sync failed for project {}: {}",
                    key,
                    result.sync_result.error_message.unwrap_or_default()
                );

                // Save checkpoint for resume (already saved in callback, but update last_synced)
                if let Some(p) = settings.find_project_mut(&key) {
                    p.sync_checkpoint = result.checkpoint;
                }
                settings.save(&self.settings_path)?;
            }
        } else {
            let enabled_projects: Vec<_> = settings
                .sync_enabled_projects()
                .iter()
                .map(|p| (p.key.clone(), p.id.clone(), p.sync_checkpoint.clone()))
                .collect();

            if enabled_projects.is_empty() {
                warn!("No projects enabled for sync");
                return Ok(());
            }

            info!("Syncing {} projects", enabled_projects.len());

            for (key, id, checkpoint) in enabled_projects {
                // Show resuming message if we have a checkpoint
                if let Some(ref cp) = checkpoint {
                    println!(
                        "Resuming sync for {} from checkpoint ({}/{} issues processed)",
                        key, cp.items_processed, cp.total_items
                    );
                }

                let settings_path_clone = settings_path.clone();
                let key_clone = key.clone();

                match use_case
                    .execute_resumable(&key, &id, checkpoint, |new_checkpoint| {
                        // Save checkpoint to settings
                        if let Ok(mut s) = Settings::load(&settings_path_clone) {
                            if let Some(p) = s.find_project_mut(&key_clone) {
                                p.sync_checkpoint = Some(new_checkpoint.clone());
                            }
                            let _ = s.save(&settings_path_clone);
                        }
                    })
                    .await
                {
                    Ok(result) => {
                        if result.sync_result.success {
                            println!(
                                "Synced {} issues for project {}",
                                result.sync_result.issues_synced, key
                            );
                            if let Some(p) = settings.find_project_mut(&key) {
                                p.last_synced = Some(Utc::now());
                                p.sync_checkpoint = None;
                            }
                        } else {
                            warn!(
                                "Sync failed for project {}: {}",
                                key,
                                result.sync_result.error_message.unwrap_or_default()
                            );
                            if let Some(p) = settings.find_project_mut(&key) {
                                p.sync_checkpoint = result.checkpoint;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to sync project {}: {}", key, e);
                    }
                }
            }

            settings.save(&self.settings_path)?;
        }

        Ok(())
    }

    pub fn handle_search(
        &self,
        query: &str,
        project: Option<String>,
        status: Option<String>,
        assignee: Option<String>,
        limit: usize,
        offset: usize,
    ) -> DomainResult<()> {
        let params = SearchParams {
            query: Some(query.to_string()),
            project_key: project,
            status,
            assignee,
            issue_type: None,
            priority: None,
            limit: Some(limit),
            offset: Some(offset),
        };

        let use_case = SearchIssuesUseCase::new(self.issue_repository.clone());
        let issues = use_case.execute(params)?;

        if issues.is_empty() {
            println!("No issues found matching your search criteria.");
            return Ok(());
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Key", "Summary", "Status", "Assignee", "Priority"]);

        for issue in &issues {
            table.add_row(vec![
                Cell::new(&issue.key),
                Cell::new(truncate(&issue.summary, 50)),
                Cell::new(issue.status.as_deref().unwrap_or("-")),
                Cell::new(issue.assignee.as_deref().unwrap_or("-")),
                Cell::new(issue.priority.as_deref().unwrap_or("-")),
            ]);
        }

        println!("{table}");
        println!("\nShowing {} issues", issues.len());

        Ok(())
    }

    pub fn handle_metadata(
        &self,
        project_key: &str,
        metadata_type: Option<String>,
    ) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;

        let project = settings
            .find_project(project_key)
            .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", project_key)))?;

        let use_case = GetProjectMetadataUseCase::new(self.metadata_repository.clone());

        let metadata = if let Some(t) = &metadata_type {
            use_case.execute_by_type(&project.id, t)?
        } else {
            use_case.execute(&project.id)?
        };

        println!("Metadata for project: {}\n", project_key);

        if !metadata.statuses.is_empty() {
            println!("Statuses ({}):", metadata.statuses.len());
            for s in &metadata.statuses {
                println!("  - {} ({})", s.name, s.category.as_deref().unwrap_or("-"));
            }
            println!();
        }

        if !metadata.priorities.is_empty() {
            println!("Priorities ({}):", metadata.priorities.len());
            for p in &metadata.priorities {
                println!("  - {}", p.name);
            }
            println!();
        }

        if !metadata.issue_types.is_empty() {
            println!("Issue Types ({}):", metadata.issue_types.len());
            for t in &metadata.issue_types {
                println!(
                    "  - {}{}",
                    t.name,
                    if t.subtask { " (subtask)" } else { "" }
                );
            }
            println!();
        }

        if !metadata.labels.is_empty() {
            println!("Labels ({}):", metadata.labels.len());
            for l in &metadata.labels {
                println!("  - {}", l.name);
            }
            println!();
        }

        if !metadata.components.is_empty() {
            println!("Components ({}):", metadata.components.len());
            for c in &metadata.components {
                println!("  - {}", c.name);
            }
            println!();
        }

        if !metadata.fix_versions.is_empty() {
            println!("Versions ({}):", metadata.fix_versions.len());
            for v in &metadata.fix_versions {
                println!(
                    "  - {}{}",
                    v.name,
                    if v.released { " (released)" } else { "" }
                );
            }
        }

        Ok(())
    }

    pub fn handle_history(
        &self,
        issue_key: &str,
        field: Option<String>,
        limit: usize,
    ) -> DomainResult<()> {
        let use_case = GetChangeHistoryUseCase::new(self.change_history_repository.clone());

        let history = use_case.execute(issue_key, field.as_deref())?;

        if history.is_empty() {
            println!("No change history found for issue: {}", issue_key);
            return Ok(());
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Date", "Field", "From", "To", "Author"]);

        for (i, item) in history.iter().enumerate() {
            if i >= limit {
                break;
            }

            table.add_row(vec![
                Cell::new(item.changed_at.format("%Y-%m-%d %H:%M").to_string()),
                Cell::new(&item.field),
                Cell::new(truncate(item.from_string.as_deref().unwrap_or("-"), 30)),
                Cell::new(truncate(item.to_string.as_deref().unwrap_or("-"), 30)),
                Cell::new(item.author_display_name.as_deref().unwrap_or("-")),
            ]);
        }

        println!("Change history for issue: {}\n", issue_key);
        println!("{table}");

        let total = use_case.count(issue_key)?;
        if total > limit {
            println!(
                "\nShowing {} of {} changes",
                limit.min(history.len()),
                total
            );
        }

        Ok(())
    }

    pub async fn handle_test_ticket(
        &self,
        project_key: &str,
        summary: &str,
        description: Option<&str>,
        issue_type: &str,
        count: usize,
    ) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;
        settings.validate()?;

        let count = count.min(10);

        let use_case = CreateTestTicketUseCase::new(self.jira_service.clone());

        for i in 1..=count {
            let ticket_summary = if count > 1 {
                format!("{} #{}", summary, i)
            } else {
                summary.to_string()
            };

            let result = use_case
                .execute(project_key, &ticket_summary, description, issue_type)
                .await?;

            let browse_url = format!(
                "{}/browse/{}",
                settings.jira.endpoint.trim_end_matches('/'),
                result.key
            );

            println!("Created ticket: {} - {}", result.key, browse_url);
        }

        Ok(())
    }

    pub fn handle_config_show(&self) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;

        println!("Current Configuration:\n");
        println!("JIRA:");
        println!("  Endpoint: {}", settings.jira.endpoint);
        println!("  Username: {}", settings.jira.username);
        println!("  API Key:  {}", mask_key(&settings.jira.api_key));
        println!("\nDatabase:");
        println!("  Path: {}", settings.database.path.display());
        println!("\nProjects: {}", settings.projects.len());

        Ok(())
    }

    pub fn handle_config_set(&self, key: &str, value: &str) -> DomainResult<()> {
        let mut settings = Settings::load(&self.settings_path)?;

        match key {
            "jira.endpoint" => settings.jira.endpoint = value.to_string(),
            "jira.username" => settings.jira.username = value.to_string(),
            "jira.api_key" => settings.jira.api_key = value.to_string(),
            "database.path" => settings.database.path = PathBuf::from(value),
            _ => {
                return Err(DomainError::Validation(format!(
                    "Unknown configuration key: {}",
                    key
                )));
            }
        }

        settings.save(&self.settings_path)?;
        println!("Updated {} = {}", key, value);

        Ok(())
    }

    pub fn handle_report(
        &self,
        project_key: Option<String>,
        interactive: bool,
        output_path: Option<String>,
    ) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;

        // Determine which projects to include
        let projects_to_report: Vec<(&str, &str, &str)> = if let Some(ref key) = project_key {
            let project = settings
                .find_project(key)
                .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", key)))?;
            vec![(&project.id, &project.key, &project.name)]
        } else {
            // All enabled projects
            let enabled = settings.sync_enabled_projects();
            if enabled.is_empty() {
                return Err(DomainError::Validation(
                    "No projects enabled for sync. Use 'jira-db project enable <KEY>' first."
                        .into(),
                ));
            }
            enabled
                .iter()
                .map(|p| (p.id.as_str(), p.key.as_str(), p.name.as_str()))
                .collect()
        };

        // Generate report data
        let use_case = GenerateReportUseCase::new(
            self.issue_repository.clone(),
            self.change_history_repository.clone(),
        );

        println!(
            "Generating report for {} project(s)...",
            projects_to_report.len()
        );
        let report_data = use_case.execute(&projects_to_report)?;

        if report_data.total_issues == 0 {
            println!("No issues found. Run 'jira-db sync' first to fetch issues.");
            return Ok(());
        }

        // Generate HTML
        let html = if interactive {
            generate_interactive_report(&report_data)
        } else {
            generate_static_report(&report_data)
        };

        // Determine output path
        let output_file = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            let reports_dir = PathBuf::from("reports");
            if !reports_dir.exists() {
                fs::create_dir_all(&reports_dir).map_err(|e| {
                    DomainError::Repository(format!("Failed to create reports directory: {}", e))
                })?;
            }

            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let suffix = if interactive { "interactive" } else { "static" };
            reports_dir.join(format!("report_{}_{}.html", timestamp, suffix))
        };

        // Ensure parent directory exists
        if let Some(parent) = output_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    DomainError::Repository(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        // Write file
        fs::write(&output_file, html)
            .map_err(|e| DomainError::Repository(format!("Failed to write report file: {}", e)))?;

        println!("Report generated successfully!");
        println!("Output: {}", output_file.display());
        println!("Total issues: {}", report_data.total_issues);
        println!(
            "Projects: {}",
            report_data
                .projects
                .iter()
                .map(|p| p.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        if interactive {
            println!("\nOpen the file in a web browser to view the interactive report.");
        }

        Ok(())
    }

    pub fn handle_snapshots_generate(&self, project_key: &str) -> DomainResult<()> {
        let settings = Settings::load(&self.settings_path)?;

        let project = settings
            .find_project(project_key)
            .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", project_key)))?;

        let use_case = GenerateSnapshotsUseCase::new(
            self.issue_repository.clone(),
            self.change_history_repository.clone(),
            self.snapshot_repository.clone(),
        );

        println!("Generating snapshots for project {}...", project_key);
        let result = use_case.execute(project_key, &project.id)?;

        println!(
            "Generated {} snapshots for {} issues",
            result.snapshots_generated, result.issues_processed
        );

        Ok(())
    }

    pub fn handle_snapshots_show(&self, issue_key: &str, version: Option<i32>) -> DomainResult<()> {
        if let Some(v) = version {
            // Show specific version
            let snapshot = self
                .snapshot_repository
                .find_by_issue_key_and_version(issue_key, v)?
                .ok_or_else(|| {
                    DomainError::NotFound(format!(
                        "Snapshot not found for issue {} version {}",
                        issue_key, v
                    ))
                })?;

            println!("Issue: {} (Version {})", issue_key, snapshot.version);
            println!(
                "Valid from: {}",
                snapshot.valid_from.format("%Y-%m-%d %H:%M:%S")
            );
            if let Some(valid_to) = snapshot.valid_to {
                println!("Valid to:   {}", valid_to.format("%Y-%m-%d %H:%M:%S"));
            } else {
                println!("Valid to:   (current)");
            }
            println!();
            println!("Summary:    {}", snapshot.summary);
            println!("Status:     {}", snapshot.status.as_deref().unwrap_or("-"));
            println!(
                "Priority:   {}",
                snapshot.priority.as_deref().unwrap_or("-")
            );
            println!(
                "Assignee:   {}",
                snapshot.assignee.as_deref().unwrap_or("-")
            );
            println!(
                "Reporter:   {}",
                snapshot.reporter.as_deref().unwrap_or("-")
            );
            println!(
                "Type:       {}",
                snapshot.issue_type.as_deref().unwrap_or("-")
            );
            println!(
                "Resolution: {}",
                snapshot.resolution.as_deref().unwrap_or("-")
            );
            if let Some(labels) = &snapshot.labels {
                println!("Labels:     {}", labels.join(", "));
            }
        } else {
            // Show all versions
            let snapshots = self.snapshot_repository.find_by_issue_key(issue_key)?;

            if snapshots.is_empty() {
                println!("No snapshots found for issue: {}", issue_key);
                println!("Run 'jira-db sync' to generate snapshots.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec![
                "Version",
                "Valid From",
                "Valid To",
                "Status",
                "Assignee",
                "Priority",
            ]);

            for snapshot in &snapshots {
                let valid_to = snapshot
                    .valid_to
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "(current)".to_string());

                table.add_row(vec![
                    Cell::new(snapshot.version),
                    Cell::new(snapshot.valid_from.format("%Y-%m-%d %H:%M").to_string()),
                    Cell::new(valid_to),
                    Cell::new(snapshot.status.as_deref().unwrap_or("-")),
                    Cell::new(snapshot.assignee.as_deref().unwrap_or("-")),
                    Cell::new(snapshot.priority.as_deref().unwrap_or("-")),
                ]);
            }

            println!("Snapshots for issue: {}\n", issue_key);
            println!("{table}");
            println!("\nTotal versions: {}", snapshots.len());
        }

        Ok(())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}
