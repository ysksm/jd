mod cli;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use log::{error, info, warn};

use jira_db_core::application::services::JiraService;
use jira_db_core::application::use_cases::{
    CreateTestTicketUseCase, GenerateReportUseCase, GenerateSnapshotsUseCase,
    GetChangeHistoryUseCase, GetProjectMetadataUseCase, SearchIssuesUseCase,
    SyncProjectListUseCase, SyncProjectUseCase,
};
use jira_db_core::chrono::{Duration, Utc};
use jira_db_core::domain::error::{DomainError, DomainResult};
use jira_db_core::domain::repositories::SearchParams;
use jira_db_core::indicatif::{ProgressBar, ProgressStyle};
use jira_db_core::infrastructure::config::{ProjectConfig, Settings, SyncCheckpoint};
use jira_db_core::infrastructure::database::{
    DatabaseFactory, DuckDbChangeHistoryRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbMetadataRepository, DuckDbProjectRepository,
    DuckDbSyncHistoryRepository, RawDataRepository,
};
use jira_db_core::infrastructure::external::jira::JiraApiClient;
use jira_db_core::report::{generate_interactive_report, generate_static_report};

use cli::{
    Cli, Commands, ConfigAction, DebugAction, EndpointAction, FieldsAction, ProjectAction,
    SnapshotsAction,
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = run().await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> DomainResult<()> {
    let cli = Cli::parse();

    let settings_path = Settings::default_path()?;

    // Handle init command separately (before settings are loaded)
    if let Commands::Init { interactive } = &cli.command {
        return handle_init_command(&settings_path, *interactive).await;
    }

    // Load settings for all other commands
    let mut settings = Settings::load(&settings_path)?;

    // Migrate legacy config if needed
    settings.migrate_legacy_config();

    // Create database factory for per-project databases
    let db_factory = Arc::new(DatabaseFactory::new(&settings));

    // Create JIRA service (DIP: implements application service trait)
    let jira_config = settings.get_jira_config().ok_or_else(|| {
        DomainError::Validation("No JIRA endpoint configured. Run 'jira-db init' first.".into())
    })?;
    let jira_service = Arc::new(JiraApiClient::new(&jira_config)?);

    // Route commands
    match cli.command {
        Commands::Init { .. } => unreachable!(), // Already handled above
        Commands::Project { action } => match action {
            ProjectAction::Init => {
                handle_project_init(&settings_path, jira_service, db_factory).await?
            }
            ProjectAction::List { verbose } => handle_project_list(&settings_path, verbose)?,
            ProjectAction::Enable { project_key } => {
                handle_project_enable(&settings_path, &project_key)?
            }
            ProjectAction::Disable { project_key } => {
                handle_project_disable(&settings_path, &project_key)?
            }
        },
        Commands::Sync { project, force: _ } => {
            handle_sync(&settings_path, db_factory, jira_service, project).await?;
        }
        Commands::Search {
            query,
            project,
            status,
            assignee,
            limit,
            offset,
        } => {
            handle_search(
                &settings_path,
                db_factory,
                &query,
                project,
                status,
                assignee,
                limit,
                offset,
            )?;
        }
        Commands::Metadata { project, r#type } => {
            handle_metadata(&settings_path, db_factory, &project, r#type)?;
        }
        Commands::History {
            issue_key,
            field,
            limit,
        } => {
            handle_history(&settings_path, db_factory, &issue_key, field, limit)?;
        }
        Commands::TestTicket {
            project,
            summary,
            description,
            issue_type,
            count,
        } => {
            handle_test_ticket(
                &settings_path,
                jira_service,
                &project,
                &summary,
                description.as_deref(),
                &issue_type,
                count,
            )
            .await?;
        }
        Commands::Config { action } => match action {
            ConfigAction::Show => handle_config_show(&settings_path)?,
            ConfigAction::Set { key, value } => handle_config_set(&settings_path, &key, &value)?,
        },
        Commands::Endpoint { action } => {
            handle_endpoint_command(&settings_path, jira_service, action).await?
        }
        Commands::Report {
            project,
            interactive,
            output,
        } => {
            handle_report(&settings_path, db_factory, project, interactive, output)?;
        }
        Commands::Embeddings {
            project,
            force,
            batch_size,
            provider,
            model,
            endpoint,
        } => {
            handle_embeddings_command(
                &settings, db_factory, project, force, batch_size, provider, model, endpoint,
            )
            .await?;
        }
        Commands::Snapshots { action } => match action {
            SnapshotsAction::Generate { project } => {
                handle_snapshots_generate(&settings_path, db_factory, &project)?;
            }
            SnapshotsAction::Show { issue_key, version } => {
                handle_snapshots_show(&settings_path, db_factory, &issue_key, version)?;
            }
        },
        Commands::Fields { action } => {
            handle_fields_command(&settings_path, db_factory, jira_service, action).await?;
        }
        Commands::Debug { action } => {
            handle_debug_command(&settings, jira_service, action).await?;
        }
    }

    Ok(())
}

async fn handle_init_command(
    settings_path: &std::path::Path,
    interactive: bool,
) -> DomainResult<()> {
    use dialoguer::{Confirm, Input};
    use jira_db_core::infrastructure::config::{DatabaseConfig, JiraConfig, JiraEndpoint};

    if Settings::exists(settings_path) {
        println!(
            "Configuration file already exists at {}",
            settings_path.display()
        );
        return Ok(());
    }

    if interactive {
        println!("JIRA-DB Configuration Setup\n");

        let name: String = Input::new()
            .with_prompt("Endpoint name (e.g., production, staging)")
            .default("default".into())
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

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

        let db_dir: String = Input::new()
            .with_prompt("Database directory")
            .default("./data".into())
            .interact_text()
            .map_err(|e| DomainError::Repository(format!("Input error: {}", e)))?;

        let jira_endpoint = JiraEndpoint {
            name: name.clone(),
            display_name: Some(name.clone()),
            endpoint: endpoint.clone(),
            username: username.clone(),
            api_key: api_key.clone(),
        };

        let settings = Settings {
            jira: None,
            jira_endpoints: vec![jira_endpoint],
            active_endpoint: Some(name),
            projects: Vec::new(),
            database: DatabaseConfig {
                path: None,
                database_dir: std::path::PathBuf::from(db_dir),
            },
            embeddings: None,
            log: None,
            sync: None,
            debug_mode: false,
        };

        println!("\nTesting JIRA connection...");
        let jira_config = JiraConfig {
            endpoint,
            username,
            api_key,
        };
        let jira_service = JiraApiClient::new(&jira_config)?;
        if let Err(e) = jira_service.test_connection().await {
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

        settings.save(settings_path)?;
        println!("\nConfiguration saved to {}", settings_path.display());
    } else {
        Settings::create_default(settings_path)?;
        println!(
            "Created default configuration file at {}",
            settings_path.display()
        );
        println!("Please edit the file to configure your JIRA connection.");
        info!("");
        info!("Next steps:");
        info!("  1. Edit the configuration file and set your JIRA endpoint credentials:");
        info!("     - jira_endpoints[0].endpoint: Your JIRA instance URL");
        info!("     - jira_endpoints[0].username: Your JIRA username/email");
        info!("     - jira_endpoints[0].api_key: Your JIRA API key");
        info!("  2. Run: jira-db project init");
    }

    Ok(())
}

async fn handle_project_init(
    settings_path: &std::path::Path,
    jira_service: Arc<JiraApiClient>,
    db_factory: Arc<DatabaseFactory>,
) -> DomainResult<()> {
    let mut settings = Settings::load(settings_path)?;
    settings.validate()?;

    // For project init, we need a temporary project repository
    // We'll use a dummy project key to create the connection
    let conn = db_factory.get_connection("_projects")?;
    let project_repository = Arc::new(DuckDbProjectRepository::new(conn));

    let use_case = SyncProjectListUseCase::new(project_repository, jira_service);

    let projects = use_case.execute().await?;

    for project in &projects {
        let project_config = ProjectConfig {
            id: project.id.clone(),
            key: project.key.clone(),
            name: project.name.clone(),
            sync_enabled: false,
            last_synced: None,
            endpoint: settings.active_endpoint.clone(), // Assign to current active endpoint
            sync_checkpoint: None,
            snapshot_checkpoint: None,
        };
        settings.upsert_project(project_config);
    }

    settings.save(settings_path)?;

    println!("Fetched {} projects from JIRA", projects.len());
    println!("Use 'jira-db project enable <PROJECT_KEY>' to enable sync for a project");

    Ok(())
}

fn handle_project_list(settings_path: &std::path::Path, verbose: bool) -> DomainResult<()> {
    use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};

    let settings = Settings::load(settings_path)?;

    if settings.projects.is_empty() {
        println!("No projects found. Run 'jira-db project init' first.");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    if verbose {
        table.set_header(vec!["Key", "Name", "Sync", "ID", "Last Synced", "Database"]);

        for project in &settings.projects {
            let last_synced = project
                .last_synced
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "-".to_string());

            let db_path = settings.get_database_path_for_project(&project.key);

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
                Cell::new(db_path.display().to_string()),
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

fn handle_project_enable(settings_path: &std::path::Path, project_key: &str) -> DomainResult<()> {
    let mut settings = Settings::load(settings_path)?;

    if let Some(project) = settings.find_project_mut(project_key) {
        project.sync_enabled = true;
        settings.save(settings_path)?;
        println!("Enabled sync for project: {}", project_key);
    } else {
        return Err(DomainError::NotFound(format!(
            "Project not found: {}",
            project_key
        )));
    }

    Ok(())
}

fn handle_project_disable(settings_path: &std::path::Path, project_key: &str) -> DomainResult<()> {
    let mut settings = Settings::load(settings_path)?;

    if let Some(project) = settings.find_project_mut(project_key) {
        project.sync_enabled = false;
        settings.save(settings_path)?;
        println!("Disabled sync for project: {}", project_key);
    } else {
        return Err(DomainError::NotFound(format!(
            "Project not found: {}",
            project_key
        )));
    }

    Ok(())
}

async fn handle_sync(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    jira_service: Arc<JiraApiClient>,
    project_key: Option<String>,
) -> DomainResult<()> {
    let settings = Settings::load(settings_path)?;
    settings.validate()?;

    if settings.projects.is_empty() {
        return Err(DomainError::Validation(
            "No projects found. Run 'jira-db project init' first.".into(),
        ));
    }

    let settings_path = settings_path.to_path_buf();

    // Get sync settings for incremental sync
    let sync_settings = settings.get_sync_settings();

    if let Some(key) = project_key {
        let project = settings
            .find_project(&key)
            .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", key)))?;

        let project_id = project.id.clone();
        let last_synced = project.last_synced;

        // Determine the checkpoint to use:
        // 1. If there's an existing sync_checkpoint (interrupted sync), use it
        // 2. If incremental sync is enabled and we have last_synced, create an incremental checkpoint
        // 3. Otherwise, do a full sync (no checkpoint)
        let checkpoint = if let Some(cp) = project.sync_checkpoint.clone() {
            // Resuming from interrupted sync
            Some(cp)
        } else if sync_settings.incremental_sync_enabled {
            if let Some(last_sync_time) = last_synced {
                // Create incremental sync checkpoint with safety margin
                // JQL only supports minute-level precision, so we subtract a safety margin
                // to ensure no data is missed
                let margin_minutes = sync_settings.incremental_sync_margin_minutes as i64;
                let incremental_start = last_sync_time - Duration::minutes(margin_minutes);
                println!(
                    "Incremental sync for {}: fetching issues updated since {} (margin: {} min)",
                    key,
                    incremental_start.format("%Y-%m-%d %H:%M:%S"),
                    margin_minutes
                );
                Some(SyncCheckpoint {
                    last_issue_updated_at: incremental_start,
                    last_issue_key: String::new(), // Empty = don't skip any issues
                    items_processed: 0,
                    total_items: 0,
                })
            } else {
                // First sync - no checkpoint
                println!("Full sync for {} (first time)", key);
                None
            }
        } else {
            // Incremental sync disabled - full sync
            println!("Full sync for {} (incremental sync disabled)", key);
            None
        };

        // Get connection for this specific project
        let conn = db_factory.get_connection(&key)?;
        let raw_conn = db_factory.get_raw_connection(&key)?;

        let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
        let change_history_repository = Arc::new(DuckDbChangeHistoryRepository::new(conn.clone()));
        let metadata_repository = Arc::new(DuckDbMetadataRepository::new(conn.clone()));
        let sync_history_repository = Arc::new(DuckDbSyncHistoryRepository::new(conn.clone()));
        let snapshot_repository = Arc::new(DuckDbIssueSnapshotRepository::new(conn));
        let raw_repository = Arc::new(RawDataRepository::new(raw_conn));

        let use_case = SyncProjectUseCase::new(
            issue_repository,
            change_history_repository,
            metadata_repository,
            sync_history_repository,
            snapshot_repository,
            jira_service.clone(),
        )
        .with_raw_repository(raw_repository);

        // Show resuming message if we have a checkpoint from interrupted sync
        if project.sync_checkpoint.is_some() {
            if let Some(ref cp) = checkpoint {
                println!(
                    "Resuming sync for {} from checkpoint ({}/{} issues processed)",
                    key, cp.items_processed, cp.total_items
                );
            }
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("[{elapsed_precise}] {spinner:.cyan} {msg}")
                .expect("Failed to create progress style"),
        );
        pb.set_message(format!("Syncing project {}...", key));

        let settings_path_clone = settings_path.clone();
        let key_clone = key.clone();

        // Use resumable sync with checkpoint saving callback
        let result = use_case
            .execute_resumable(&key, &project_id, checkpoint, move |new_checkpoint| {
                // Update progress bar with checkpoint info
                pb.set_message(format!(
                    "Syncing project {}... ({}/{} issues)",
                    key_clone, new_checkpoint.items_processed, new_checkpoint.total_items
                ));

                // Save checkpoint to settings
                if let Ok(mut s) = Settings::load(&settings_path_clone) {
                    if let Some(p) = s.find_project_mut(&key_clone) {
                        p.sync_checkpoint = Some(new_checkpoint.clone());
                    }
                    let _ = s.save(&settings_path_clone);
                }
            })
            .await?;

        if result.sync_result.success {
            println!(
                "Synced {} issues ({} history items) for project {}",
                result.sync_result.issues_synced, result.sync_result.history_items_synced, key
            );

            // Clear checkpoint on success
            let mut settings = Settings::load(&settings_path)?;
            if let Some(p) = settings.find_project_mut(&key) {
                p.last_synced = Some(Utc::now());
                p.sync_checkpoint = None;
            }
            settings.save(&settings_path)?;
        } else {
            println!(
                "Sync failed for project {}: {}",
                key,
                result.sync_result.error_message.unwrap_or_default()
            );

            // Save checkpoint for resume
            let mut settings = Settings::load(&settings_path)?;
            if let Some(p) = settings.find_project_mut(&key) {
                p.sync_checkpoint = result.checkpoint;
            }
            settings.save(&settings_path)?;
        }
    } else {
        // Collect project info including last_synced for incremental sync
        let enabled_projects: Vec<_> = settings
            .sync_enabled_projects()
            .iter()
            .map(|p| {
                (
                    p.key.clone(),
                    p.id.clone(),
                    p.sync_checkpoint.clone(),
                    p.last_synced,
                )
            })
            .collect();

        if enabled_projects.is_empty() {
            warn!("No projects enabled for sync");
            return Ok(());
        }

        info!("Syncing {} projects", enabled_projects.len());

        for (key, id, existing_checkpoint, last_synced) in enabled_projects {
            // Determine the checkpoint to use (same logic as single project)
            let checkpoint = if let Some(cp) = existing_checkpoint.clone() {
                // Resuming from interrupted sync
                Some(cp)
            } else if sync_settings.incremental_sync_enabled {
                if let Some(last_sync_time) = last_synced {
                    // Create incremental sync checkpoint with safety margin
                    let margin_minutes = sync_settings.incremental_sync_margin_minutes as i64;
                    let incremental_start = last_sync_time - Duration::minutes(margin_minutes);
                    println!(
                        "Incremental sync for {}: fetching issues updated since {} (margin: {} min)",
                        key,
                        incremental_start.format("%Y-%m-%d %H:%M:%S"),
                        margin_minutes
                    );
                    Some(SyncCheckpoint {
                        last_issue_updated_at: incremental_start,
                        last_issue_key: String::new(),
                        items_processed: 0,
                        total_items: 0,
                    })
                } else {
                    println!("Full sync for {} (first time)", key);
                    None
                }
            } else {
                println!("Full sync for {} (incremental sync disabled)", key);
                None
            };

            // Get connection for this specific project
            let conn = db_factory.get_connection(&key)?;
            let raw_conn = db_factory.get_raw_connection(&key)?;

            let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
            let change_history_repository =
                Arc::new(DuckDbChangeHistoryRepository::new(conn.clone()));
            let metadata_repository = Arc::new(DuckDbMetadataRepository::new(conn.clone()));
            let sync_history_repository = Arc::new(DuckDbSyncHistoryRepository::new(conn.clone()));
            let snapshot_repository = Arc::new(DuckDbIssueSnapshotRepository::new(conn));
            let raw_repository = Arc::new(RawDataRepository::new(raw_conn));

            let use_case = SyncProjectUseCase::new(
                issue_repository,
                change_history_repository,
                metadata_repository,
                sync_history_repository,
                snapshot_repository,
                jira_service.clone(),
            )
            .with_raw_repository(raw_repository);

            // Show resuming message if we have a checkpoint from interrupted sync
            if existing_checkpoint.is_some() {
                if let Some(ref cp) = checkpoint {
                    println!(
                        "Resuming sync for {} from checkpoint ({}/{} issues processed)",
                        key, cp.items_processed, cp.total_items
                    );
                }
            }

            let settings_path_clone = settings_path.clone();
            let key_clone = key.clone();

            match use_case
                .execute_resumable(&key, &id, checkpoint, move |new_checkpoint| {
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
                        let mut settings = Settings::load(&settings_path)?;
                        if let Some(p) = settings.find_project_mut(&key) {
                            p.last_synced = Some(Utc::now());
                            p.sync_checkpoint = None;
                        }
                        settings.save(&settings_path)?;
                    } else {
                        warn!(
                            "Sync failed for project {}: {}",
                            key,
                            result.sync_result.error_message.unwrap_or_default()
                        );
                        let mut settings = Settings::load(&settings_path)?;
                        if let Some(p) = settings.find_project_mut(&key) {
                            p.sync_checkpoint = result.checkpoint;
                        }
                        settings.save(&settings_path)?;
                    }
                }
                Err(e) => {
                    warn!("Failed to sync project {}: {}", key, e);
                }
            }
        }
    }

    Ok(())
}

fn handle_search(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    query: &str,
    project: Option<String>,
    status: Option<String>,
    assignee: Option<String>,
    limit: usize,
    offset: usize,
) -> DomainResult<()> {
    use comfy_table::{Cell, Table, presets::UTF8_FULL};

    let settings = Settings::load(settings_path)?;

    let params = SearchParams {
        query: Some(query.to_string()),
        project_key: project.clone(),
        status,
        assignee,
        issue_type: None,
        priority: None,
        team: None,
        limit: Some(limit),
        offset: Some(offset),
    };

    // Determine which projects to search
    let projects_to_search: Vec<String> = if let Some(ref key) = project {
        // Verify project exists
        if settings.find_project(key).is_none() {
            return Err(DomainError::NotFound(format!("Project not found: {}", key)));
        }
        vec![key.clone()]
    } else {
        // Search all enabled projects
        settings
            .sync_enabled_projects()
            .iter()
            .map(|p| p.key.clone())
            .collect()
    };

    if projects_to_search.is_empty() {
        println!("No projects to search. Enable sync for a project first.");
        return Ok(());
    }

    let mut all_issues = Vec::new();

    for project_key in &projects_to_search {
        let conn = match db_factory.get_connection(project_key) {
            Ok(c) => c,
            Err(_) => continue, // Skip projects without database
        };

        let issue_repository = Arc::new(DuckDbIssueRepository::new(conn));
        let use_case = SearchIssuesUseCase::new(issue_repository);

        match use_case.execute(params.clone()) {
            Ok(issues) => all_issues.extend(issues),
            Err(_) => continue, // Skip on errors
        }
    }

    if all_issues.is_empty() {
        println!("No issues found matching your search criteria.");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Key", "Summary", "Status", "Assignee", "Priority"]);

    for issue in &all_issues {
        table.add_row(vec![
            Cell::new(&issue.key),
            Cell::new(truncate(&issue.summary, 50)),
            Cell::new(issue.status.as_deref().unwrap_or("-")),
            Cell::new(issue.assignee.as_deref().unwrap_or("-")),
            Cell::new(issue.priority.as_deref().unwrap_or("-")),
        ]);
    }

    println!("{table}");
    println!("\nShowing {} issues", all_issues.len());

    Ok(())
}

fn handle_metadata(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    project_key: &str,
    metadata_type: Option<String>,
) -> DomainResult<()> {
    let settings = Settings::load(settings_path)?;

    let project = settings
        .find_project(project_key)
        .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", project_key)))?;

    let conn = db_factory.get_connection(project_key)?;
    let metadata_repository = Arc::new(DuckDbMetadataRepository::new(conn));

    let use_case = GetProjectMetadataUseCase::new(metadata_repository);

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

fn handle_history(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    issue_key: &str,
    field: Option<String>,
    limit: usize,
) -> DomainResult<()> {
    use comfy_table::{Cell, Table, presets::UTF8_FULL};

    // Extract project key from issue key (e.g., "PROJ-123" -> "PROJ")
    let project_key = issue_key
        .split('-')
        .next()
        .ok_or_else(|| DomainError::Validation(format!("Invalid issue key: {}", issue_key)))?;

    let settings = Settings::load(settings_path)?;

    // Verify project exists
    if settings.find_project(project_key).is_none() {
        return Err(DomainError::NotFound(format!(
            "Project not found: {}",
            project_key
        )));
    }

    let conn = db_factory.get_connection(project_key)?;
    let change_history_repository = Arc::new(DuckDbChangeHistoryRepository::new(conn));

    let use_case = GetChangeHistoryUseCase::new(change_history_repository.clone());

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

async fn handle_test_ticket(
    settings_path: &std::path::Path,
    jira_service: Arc<JiraApiClient>,
    project_key: &str,
    summary: &str,
    description: Option<&str>,
    issue_type: &str,
    count: usize,
) -> DomainResult<()> {
    let settings = Settings::load(settings_path)?;
    settings.validate()?;

    let count = count.min(10);

    let use_case = CreateTestTicketUseCase::new(jira_service);

    for i in 1..=count {
        let ticket_summary = if count > 1 {
            format!("{} #{}", summary, i)
        } else {
            summary.to_string()
        };

        let result = use_case
            .execute(project_key, &ticket_summary, description, issue_type)
            .await?;

        let jira_endpoint = settings
            .get_jira_config()
            .map(|c| c.endpoint)
            .unwrap_or_default();
        let browse_url = format!(
            "{}/browse/{}",
            jira_endpoint.trim_end_matches('/'),
            result.key
        );

        println!("Created ticket: {} - {}", result.key, browse_url);
    }

    Ok(())
}

fn handle_config_show(settings_path: &std::path::Path) -> DomainResult<()> {
    use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};

    let mut settings = Settings::load(settings_path)?;
    settings.migrate_legacy_config();

    println!("Current Configuration:\n");

    // Show endpoints
    println!("JIRA Endpoints:");
    if settings.jira_endpoints.is_empty() {
        println!("  (no endpoints configured)");
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Name", "URL", "Username", "Active"]);

        for ep in &settings.jira_endpoints {
            let is_active = settings.active_endpoint.as_deref() == Some(&ep.name);
            table.add_row(vec![
                Cell::new(&ep.name),
                Cell::new(&ep.endpoint),
                Cell::new(&ep.username),
                if is_active {
                    Cell::new("*").fg(Color::Green)
                } else {
                    Cell::new("")
                },
            ]);
        }
        println!("{table}");
    }

    println!("\nDatabase:");
    println!("  Directory: {}", settings.database.database_dir.display());
    println!("  (Each project has its own database file: <project_key>.duckdb)");
    println!("\nProjects: {}", settings.projects.len());
    println!(
        "\nDebug Mode: {}",
        if settings.debug_mode {
            "enabled"
        } else {
            "disabled"
        }
    );

    Ok(())
}

fn handle_config_set(settings_path: &std::path::Path, key: &str, value: &str) -> DomainResult<()> {
    let mut settings = Settings::load(settings_path)?;
    settings.migrate_legacy_config();

    match key {
        "active_endpoint" => {
            if settings.set_active_endpoint(value) {
                println!("Set active endpoint to '{}'", value);
            } else {
                return Err(DomainError::Validation(format!(
                    "Endpoint '{}' not found",
                    value
                )));
            }
        }
        "database.database_dir" => settings.database.database_dir = PathBuf::from(value),
        "debug_mode" => {
            settings.debug_mode = value.to_lowercase() == "true" || value == "1";
        }
        _ => {
            return Err(DomainError::Validation(format!(
                "Unknown configuration key: {}. Use 'endpoint' commands to manage JIRA endpoints.",
                key
            )));
        }
    }

    settings.save(settings_path)?;
    println!("Updated {} = {}", key, value);

    Ok(())
}

fn handle_report(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    project_key: Option<String>,
    interactive: bool,
    output_path: Option<String>,
) -> DomainResult<()> {
    use std::fs;

    let settings = Settings::load(settings_path)?;

    // Determine which projects to include
    let projects_to_report: Vec<(String, String, String)> = if let Some(ref key) = project_key {
        let project = settings
            .find_project(key)
            .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", key)))?;
        vec![(
            project.id.clone(),
            project.key.clone(),
            project.name.clone(),
        )]
    } else {
        // All enabled projects
        let enabled = settings.sync_enabled_projects();
        if enabled.is_empty() {
            return Err(DomainError::Validation(
                "No projects enabled for sync. Use 'jira-db project enable <KEY>' first.".into(),
            ));
        }
        enabled
            .iter()
            .map(|p| (p.id.clone(), p.key.clone(), p.name.clone()))
            .collect()
    };

    println!(
        "Generating report for {} project(s)...",
        projects_to_report.len()
    );

    // Collect report data from all projects
    let mut all_report_data: Option<jira_db_core::application::use_cases::ReportData> = None;

    for (id, key, name) in &projects_to_report {
        let conn = match db_factory.get_connection(key) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
        let change_history_repository = Arc::new(DuckDbChangeHistoryRepository::new(conn));

        let use_case = GenerateReportUseCase::new(issue_repository, change_history_repository);

        let project_tuple = vec![(id.as_str(), key.as_str(), name.as_str())];
        match use_case.execute(&project_tuple) {
            Ok(data) => {
                if let Some(ref mut all_data) = all_report_data {
                    // Merge data
                    all_data.total_issues += data.total_issues;
                    all_data.projects.extend(data.projects);
                } else {
                    all_report_data = Some(data);
                }
            }
            Err(_) => continue,
        }
    }

    let report_data = all_report_data.ok_or_else(|| {
        DomainError::Validation("No data found. Run 'jira-db sync' first.".into())
    })?;

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

async fn handle_embeddings_command(
    settings: &Settings,
    db_factory: Arc<DatabaseFactory>,
    project: Option<String>,
    force: bool,
    batch_size: usize,
    cli_provider: Option<String>,
    cli_model: Option<String>,
    cli_endpoint: Option<String>,
) -> DomainResult<()> {
    use jira_db_core::application::use_cases::{
        EmbeddingGenerationConfig, GenerateEmbeddingsUseCase,
    };
    use jira_db_core::infrastructure::database::EmbeddingsRepository;
    use jira_db_core::infrastructure::external::embeddings::{
        EmbeddingProviderType, ProviderConfig, create_provider,
    };

    // Project is required for embeddings
    let project_key = project.ok_or_else(|| {
        DomainError::Validation(
            "Project key is required for embeddings. Use --project <KEY>".into(),
        )
    })?;

    // Verify project exists
    if settings.find_project(&project_key).is_none() {
        return Err(DomainError::NotFound(format!(
            "Project not found: {}",
            project_key
        )));
    }

    // Determine provider type (CLI > settings > default)
    let provider_str = cli_provider
        .or_else(|| settings.embeddings.as_ref().map(|e| e.provider.clone()))
        .unwrap_or_else(|| "openai".to_string());

    let provider_type: EmbeddingProviderType = provider_str.parse()?;

    // Get model (CLI > settings > provider default)
    let model = cli_model.or_else(|| settings.embeddings.as_ref().map(|e| e.model.clone()));

    // Get endpoint (CLI > settings)
    let endpoint = cli_endpoint.or_else(|| {
        settings
            .embeddings
            .as_ref()
            .and_then(|e| e.endpoint.clone())
    });

    // Get API key from settings or environment
    let api_key = settings
        .embeddings
        .as_ref()
        .and_then(|e| e.get_api_key().cloned());

    // Build provider configuration
    let provider_config = ProviderConfig {
        provider: provider_type,
        api_key,
        model: model.clone(),
        endpoint,
    };

    // Display what we're using
    let display_model = model.clone().unwrap_or_else(|| match provider_type {
        EmbeddingProviderType::OpenAI => "text-embedding-3-small".to_string(),
        EmbeddingProviderType::Ollama => "nomic-embed-text".to_string(),
        EmbeddingProviderType::Cohere => "embed-multilingual-v3.0".to_string(),
    });

    println!("Generating embeddings using:");
    println!("  Provider: {}", provider_type);
    println!("  Model:    {}", display_model);
    if let Some(ref ep) = provider_config.endpoint {
        println!("  Endpoint: {}", ep);
    }
    println!();

    // Create embedding provider
    let embedding_provider = create_provider(provider_config)?;

    // Get connection for this project
    let conn = db_factory.get_connection(&project_key)?;
    let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
    let embeddings_repository = Arc::new(EmbeddingsRepository::new(conn));

    // Create config
    let config = EmbeddingGenerationConfig {
        batch_size,
        force_regenerate: force,
    };

    // Create and execute use case
    let use_case = GenerateEmbeddingsUseCase::new(
        issue_repository,
        embeddings_repository,
        Arc::new(embedding_provider),
        config,
    );

    let result = use_case.execute(Some(&project_key)).await?;

    // Print results
    println!("\nEmbedding Generation Results:");
    println!("  Total issues:        {}", result.total_issues);
    println!("  Embeddings generated: {}", result.embeddings_generated);
    println!("  Embeddings skipped:   {}", result.embeddings_skipped);
    println!("  Errors:              {}", result.errors);
    println!("  Total time:          {:.2}s", result.duration_secs);
    println!("\nTiming breakdown:");
    println!(
        "  Fetch issues:        {:.2}s",
        result.timing.fetch_issues_secs
    );
    println!(
        "  Embedding API:       {:.2}s",
        result.timing.embedding_api_secs
    );
    println!(
        "  Store embeddings:    {:.2}s",
        result.timing.store_embeddings_secs
    );

    Ok(())
}

fn handle_snapshots_generate(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    project_key: &str,
) -> DomainResult<()> {
    let settings = Settings::load(settings_path)?;

    let project = settings
        .find_project(project_key)
        .ok_or_else(|| DomainError::NotFound(format!("Project not found: {}", project_key)))?;

    let conn = db_factory.get_connection(project_key)?;
    let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
    let change_history_repository = Arc::new(DuckDbChangeHistoryRepository::new(conn.clone()));
    let snapshot_repository = Arc::new(DuckDbIssueSnapshotRepository::new(conn));

    let use_case = GenerateSnapshotsUseCase::new(
        issue_repository,
        change_history_repository,
        snapshot_repository,
    );

    println!("Generating snapshots for project {}...", project_key);
    let result = use_case.execute(project_key, &project.id)?;

    println!(
        "Generated {} snapshots for {} issues",
        result.snapshots_generated, result.issues_processed
    );

    Ok(())
}

fn handle_snapshots_show(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    issue_key: &str,
    version: Option<i32>,
) -> DomainResult<()> {
    use comfy_table::{Cell, Table, presets::UTF8_FULL};
    use jira_db_core::domain::repositories::IssueSnapshotRepository;

    // Extract project key from issue key
    let project_key = issue_key
        .split('-')
        .next()
        .ok_or_else(|| DomainError::Validation(format!("Invalid issue key: {}", issue_key)))?;

    let settings = Settings::load(settings_path)?;

    // Verify project exists
    if settings.find_project(project_key).is_none() {
        return Err(DomainError::NotFound(format!(
            "Project not found: {}",
            project_key
        )));
    }

    let conn = db_factory.get_connection(project_key)?;
    let snapshot_repository = DuckDbIssueSnapshotRepository::new(conn);

    if let Some(v) = version {
        // Show specific version
        let snapshot = snapshot_repository
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
        let snapshots = snapshot_repository.find_by_issue_key(issue_key)?;

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

async fn handle_fields_command(
    settings_path: &std::path::Path,
    db_factory: Arc<DatabaseFactory>,
    jira_service: Arc<JiraApiClient>,
    action: FieldsAction,
) -> DomainResult<()> {
    use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
    use jira_db_core::application::use_cases::SyncFieldsUseCase;
    use jira_db_core::infrastructure::database::{
        DuckDbFieldRepository, DuckDbIssuesExpandedRepository,
    };

    let settings = Settings::load(settings_path)?;

    // Fields are project-specific, so we need a project key
    let project_key = match &action {
        FieldsAction::Expand { project } => project.clone(),
        FieldsAction::Full { project } => project.clone(),
        FieldsAction::Sync | FieldsAction::List { .. } => {
            // For sync and list, use first enabled project or require --project
            settings
                .sync_enabled_projects()
                .first()
                .map(|p| Some(p.key.clone()))
                .unwrap_or(None)
        }
    };

    let project_key = project_key.ok_or_else(|| {
        DomainError::Validation(
            "No project specified. Use --project <KEY> or enable a project.".into(),
        )
    })?;

    let conn = db_factory.get_connection(&project_key)?;
    let field_repo = Arc::new(DuckDbFieldRepository::new(conn.clone()));
    let expanded_repo = Arc::new(DuckDbIssuesExpandedRepository::new(conn));

    match action {
        FieldsAction::Sync => {
            println!("Fetching field definitions from JIRA...");
            let use_case =
                SyncFieldsUseCase::new(jira_service, field_repo.clone(), expanded_repo.clone());

            let count = use_case.sync_fields().await?;
            println!("Synced {} field definitions", count);
        }

        FieldsAction::List { custom, navigable } => {
            let fields = if navigable {
                field_repo.find_navigable()?
            } else {
                field_repo.find_all()?
            };

            if fields.is_empty() {
                println!("No field definitions found. Run 'jira-db fields sync' first.");
                return Ok(());
            }

            let filtered: Vec<_> = if custom {
                fields.into_iter().filter(|f| f.custom).collect()
            } else {
                fields
            };

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "Type", "Custom", "Navigable"]);

            for field in &filtered {
                table.add_row(vec![
                    Cell::new(&field.id),
                    Cell::new(&field.name),
                    Cell::new(field.schema_type.as_deref().unwrap_or("-")),
                    if field.custom {
                        Cell::new("Yes").fg(Color::Yellow)
                    } else {
                        Cell::new("No")
                    },
                    if field.navigable {
                        Cell::new("Yes").fg(Color::Green)
                    } else {
                        Cell::new("No").fg(Color::DarkGrey)
                    },
                ]);
            }

            println!("{table}");
            println!("\nTotal: {} fields", filtered.len());
        }

        FieldsAction::Expand { project: _ } => {
            println!("Adding columns based on field definitions...");
            let use_case = SyncFieldsUseCase::new(jira_service, field_repo, expanded_repo.clone());

            let added = use_case.add_columns()?;
            if !added.is_empty() {
                println!("Added {} new columns: {}", added.len(), added.join(", "));
            }

            println!("Expanding issues from raw_data...");
            let count = use_case.expand_issues(Some(&project_key))?;
            println!("Expanded {} issues into issues_expanded table", count);
        }

        FieldsAction::Full { project: _ } => {
            println!("Running full field sync and expansion...\n");
            let use_case = SyncFieldsUseCase::new(jira_service, field_repo, expanded_repo);

            let result = use_case.execute(Some(&project_key)).await?;

            println!("Results:");
            println!("  Fields synced:    {}", result.fields_synced);
            println!("  Columns added:    {}", result.columns_added);
            println!("  Issues expanded:  {}", result.issues_expanded);
        }
    }

    Ok(())
}

async fn handle_debug_command(
    settings: &Settings,
    jira_service: Arc<JiraApiClient>,
    action: DebugAction,
) -> DomainResult<()> {
    use comfy_table::{Cell, Table, presets::UTF8_FULL};
    use jira_db_core::application::use_cases::{CreateTestTicketUseCase, TransitionIssueUseCase};

    // Check if debug mode is enabled
    if !settings.debug_mode {
        match action {
            DebugAction::Status => {
                println!("Debug mode: disabled");
                println!("\nTo enable debug mode, set debug_mode: true in settings.json");
                return Ok(());
            }
            _ => {
                return Err(DomainError::Validation(
                    "Debug mode is not enabled. Set debug_mode: true in settings.json".into(),
                ));
            }
        }
    }

    settings.validate()?;

    match action {
        DebugAction::Status => {
            println!("Debug mode: enabled");
            println!("\nAvailable debug commands:");
            println!("  create-issues     Create test issues in JIRA");
            println!("  list-transitions  List available transitions for an issue");
            println!("  transition-issue  Transition a single issue");
            println!("  bulk-transition   Transition multiple issues");
        }

        DebugAction::CreateIssues {
            project,
            count,
            issue_type,
            summary,
            description,
        } => {
            let count = count.min(100).max(1);

            println!(
                "Creating {} {} issue(s) in project {}...",
                count, issue_type, project
            );

            let use_case = CreateTestTicketUseCase::new(jira_service);

            for i in 1..=count {
                let ticket_summary = if count > 1 {
                    format!("{} #{}", summary, i)
                } else {
                    summary.clone()
                };

                let result = use_case
                    .execute(
                        &project,
                        &ticket_summary,
                        description.as_deref(),
                        &issue_type,
                    )
                    .await?;

                let jira_endpoint = settings
                    .get_jira_config()
                    .map(|c| c.endpoint)
                    .unwrap_or_default();
                let browse_url = format!(
                    "{}/browse/{}",
                    jira_endpoint.trim_end_matches('/'),
                    result.key
                );

                println!("[{}/{}] Created: {} - {}", i, count, result.key, browse_url);
            }

            println!("\nSuccessfully created {} issue(s)", count);
        }

        DebugAction::ListTransitions { issue_key } => {
            println!("Fetching available transitions for {}...\n", issue_key);

            let use_case = TransitionIssueUseCase::new(jira_service);
            let transitions = use_case.get_transitions(&issue_key).await?;

            if transitions.is_empty() {
                println!("No transitions available for this issue.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "To Status", "Category"]);

            for t in &transitions {
                table.add_row(vec![
                    Cell::new(&t.id),
                    Cell::new(&t.name),
                    Cell::new(&t.to_status),
                    Cell::new(t.to_status_category.as_deref().unwrap_or("-")),
                ]);
            }

            println!("{table}");
            println!("\nUse the ID with 'transition-issue' or 'bulk-transition' command");
        }

        DebugAction::TransitionIssue {
            issue_key,
            transition_id,
        } => {
            println!(
                "Transitioning {} with transition ID {}...",
                issue_key, transition_id
            );

            let use_case = TransitionIssueUseCase::new(jira_service);
            use_case.transition(&issue_key, &transition_id).await?;

            println!("Successfully transitioned {}", issue_key);
        }

        DebugAction::BulkTransition {
            issues,
            transition_id,
        } => {
            let issue_keys: Vec<String> = issues
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if issue_keys.is_empty() {
                return Err(DomainError::Validation("No issue keys provided".into()));
            }

            println!(
                "Transitioning {} issue(s) with transition ID {}...\n",
                issue_keys.len(),
                transition_id
            );

            let use_case = TransitionIssueUseCase::new(jira_service);
            let results = use_case
                .transition_multiple(&issue_keys, &transition_id)
                .await;

            let mut success_count = 0;
            let mut error_count = 0;

            for result in &results {
                if result.success {
                    println!("  [OK] {}", result.issue_key);
                    success_count += 1;
                } else {
                    println!(
                        "  [FAILED] {} - {}",
                        result.issue_key,
                        result.error.as_deref().unwrap_or("Unknown error")
                    );
                    error_count += 1;
                }
            }

            println!(
                "\nCompleted: {} success, {} failed",
                success_count, error_count
            );
        }
    }

    Ok(())
}

async fn handle_endpoint_command(
    settings_path: &std::path::Path,
    _jira_service: Arc<JiraApiClient>,
    action: EndpointAction,
) -> DomainResult<()> {
    use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
    use jira_db_core::infrastructure::config::JiraEndpoint;

    let mut settings = Settings::load(settings_path)?;
    settings.migrate_legacy_config();

    match action {
        EndpointAction::List => {
            if settings.jira_endpoints.is_empty() {
                println!("No JIRA endpoints configured.");
                println!("Use 'jira-db endpoint add' to add an endpoint.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["Name", "Display Name", "URL", "Username", "Active"]);

            for ep in &settings.jira_endpoints {
                let is_active = settings.active_endpoint.as_deref() == Some(&ep.name);
                table.add_row(vec![
                    Cell::new(&ep.name),
                    Cell::new(ep.display_name.as_deref().unwrap_or("-")),
                    Cell::new(&ep.endpoint),
                    Cell::new(&ep.username),
                    if is_active {
                        Cell::new("*").fg(Color::Green)
                    } else {
                        Cell::new("")
                    },
                ]);
            }

            println!("JIRA Endpoints:\n");
            println!("{table}");
            println!("\nTotal: {} endpoint(s)", settings.jira_endpoints.len());
        }

        EndpointAction::Add {
            name,
            url,
            username,
            api_key,
            display_name,
        } => {
            // Check if name already exists
            if settings.get_endpoint(&name).is_some() {
                return Err(DomainError::Validation(format!(
                    "Endpoint '{}' already exists. Use a different name or remove it first.",
                    name
                )));
            }

            let endpoint = JiraEndpoint {
                name: name.clone(),
                display_name,
                endpoint: url,
                username,
                api_key,
            };

            settings.add_endpoint(endpoint);

            // Set as active if this is the first endpoint
            if settings.jira_endpoints.len() == 1 {
                settings.set_active_endpoint(&name);
            }

            settings.save(settings_path)?;
            println!("Added endpoint '{}'", name);

            if settings.active_endpoint.as_deref() == Some(&name) {
                println!("Set as active endpoint.");
            }
        }

        EndpointAction::Remove { name } => {
            if settings.get_endpoint(&name).is_none() {
                return Err(DomainError::NotFound(format!(
                    "Endpoint '{}' not found",
                    name
                )));
            }

            if settings.remove_endpoint(&name) {
                settings.save(settings_path)?;
                println!("Removed endpoint '{}'", name);
            }
        }

        EndpointAction::SetActive { name } => {
            if settings.set_active_endpoint(&name) {
                settings.save(settings_path)?;
                println!("Set '{}' as active endpoint", name);
            } else {
                return Err(DomainError::NotFound(format!(
                    "Endpoint '{}' not found",
                    name
                )));
            }
        }

        EndpointAction::Show { name } => {
            let endpoint = settings
                .get_endpoint(&name)
                .ok_or_else(|| DomainError::NotFound(format!("Endpoint '{}' not found", name)))?;

            let is_active = settings.active_endpoint.as_deref() == Some(&name);

            println!("Endpoint: {}", endpoint.name);
            println!(
                "Display Name: {}",
                endpoint.display_name.as_deref().unwrap_or("-")
            );
            println!("URL:      {}", endpoint.endpoint);
            println!("Username: {}", endpoint.username);
            println!("API Key:  {}", mask_key(&endpoint.api_key));
            println!("Active:   {}", if is_active { "Yes" } else { "No" });
        }

        EndpointAction::Test { name } => {
            let endpoint_name = name.or_else(|| settings.active_endpoint.clone());

            let endpoint_name = endpoint_name.ok_or_else(|| {
                DomainError::Validation("No endpoint specified and no active endpoint set".into())
            })?;

            let endpoint = settings.get_endpoint(&endpoint_name).ok_or_else(|| {
                DomainError::NotFound(format!("Endpoint '{}' not found", endpoint_name))
            })?;

            println!("Testing connection to '{}'...", endpoint.name);
            println!("URL: {}", endpoint.endpoint);

            let test_client = JiraApiClient::new(&endpoint.to_jira_config())?;
            match test_client.test_connection().await {
                Ok(_) => {
                    println!("Connection successful!");
                }
                Err(e) => {
                    println!("Connection failed: {}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(())
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
