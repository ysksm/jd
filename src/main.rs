mod cli;
mod config;
mod db;
mod error;
mod jira;
mod sync;

use clap::Parser;
use crate::cli::{Cli, Commands, ConfigAction, ProjectAction};
use crate::config::Settings;
use crate::db::Database;
use crate::error::{JiraDbError, Result};
use crate::jira::JiraClient;
use crate::sync::SyncManager;
use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = run().await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => handle_init().await,
        Commands::Project { action } => handle_project(action).await,
        Commands::Sync { project, force } => handle_sync(project, force).await,
        Commands::Config { action } => handle_config(action).await,
        Commands::Search {
            query,
            project,
            status,
        } => handle_search(query, project, status).await,
    }
}

async fn handle_init() -> Result<()> {
    info!("Initializing jira-db configuration...");

    let settings_path = Settings::default_path()?;

    if Settings::exists(&settings_path) {
        return Err(JiraDbError::Config(format!(
            "Configuration file already exists at: {}",
            settings_path.display()
        )));
    }

    let _settings = Settings::create_default(&settings_path)?;

    info!("Created configuration file at: {}", settings_path.display());
    info!("");
    info!("Next steps:");
    info!("  1. Edit the configuration file and set your JIRA credentials:");
    info!("     - endpoint: Your JIRA instance URL");
    info!("     - username: Your JIRA username/email");
    info!("     - api_key: Your JIRA API key");
    info!("  2. Run: jira-db project init");

    Ok(())
}

async fn handle_project(action: ProjectAction) -> Result<()> {
    let settings_path = Settings::default_path()?;

    match action {
        ProjectAction::Init => {
            info!("Initializing project list from JIRA...");

            let mut settings = Settings::load(&settings_path)?;
            settings.validate()?;

            info!("Connecting to JIRA...");
            let client = JiraClient::new(&settings.jira)?;

            client.test_connection().await?;
            info!("Connected successfully!");

            let db = Database::new(&settings.database.path)?;
            let sync_manager = SyncManager::new(client, db);

            sync_manager.sync_project_list(&mut settings).await?;
            settings.save(&settings_path)?;

            info!("");
            info!("Project list initialized successfully!");
            info!("Run 'jira-db project list' to see all projects");
            info!("Use 'jira-db project enable <PROJECT_KEY>' to enable sync for specific projects");

            Ok(())
        }
        ProjectAction::List { verbose } => handle_project_list(verbose).await,
        ProjectAction::Enable { project_key } => {
            let mut settings = Settings::load(&settings_path)?;

            if let Some(project) = settings.find_project_mut(&project_key) {
                project.sync_enabled = true;
                settings.save(&settings_path)?;
                info!("Enabled sync for project: {}", project_key);
            } else {
                return Err(JiraDbError::ProjectNotFound(project_key));
            }

            Ok(())
        }
        ProjectAction::Disable { project_key } => {
            let mut settings = Settings::load(&settings_path)?;

            if let Some(project) = settings.find_project_mut(&project_key) {
                project.sync_enabled = false;
                settings.save(&settings_path)?;
                info!("Disabled sync for project: {}", project_key);
            } else {
                return Err(JiraDbError::ProjectNotFound(project_key));
            }

            Ok(())
        }
    }
}

async fn handle_sync(project_key: Option<String>, _force: bool) -> Result<()> {
    let settings_path = Settings::default_path()?;
    let mut settings = Settings::load(&settings_path)?;

    settings.validate()?;

    // Check if projects are initialized
    if settings.projects.is_empty() {
        return Err(JiraDbError::Config(
            "No projects found. Please run 'jira-db project init' first.".into(),
        ));
    }

    info!("Connecting to JIRA...");
    let client = JiraClient::new(&settings.jira)?;

    client.test_connection().await?;
    info!("Connected successfully!");

    let db = Database::new(&settings.database.path)?;
    let sync_manager = SyncManager::new(client, db);

    if let Some(key) = project_key {
        // Sync specific project
        sync_manager.sync_project(&key, &mut settings).await?;
        settings.save(&settings_path)?;
    } else {
        // Sync all enabled projects
        sync_manager.sync_all(&mut settings).await?;
        settings.save(&settings_path)?;
    }

    Ok(())
}

async fn handle_project_list(verbose: bool) -> Result<()> {
    let settings_path = Settings::default_path()?;
    let settings = Settings::load(&settings_path)?;

    if settings.projects.is_empty() {
        info!("No projects found. Run 'jira-db project init' to fetch projects.");
        return Ok(());
    }

    info!("Projects:");
    info!("");

    for project in &settings.projects {
        if verbose {
            info!("  {} ({})", project.key, project.name);
            info!("    ID: {}", project.id);
            info!("    Sync enabled: {}", project.sync_enabled);
            if let Some(last_synced) = project.last_synced {
                info!("    Last synced: {}", last_synced.format("%Y-%m-%d %H:%M:%S UTC"));
            } else {
                info!("    Last synced: Never");
            }
            info!("");
        } else {
            let sync_status = if project.sync_enabled { "✓" } else { " " };
            info!("  [{}] {} - {}", sync_status, project.key, project.name);
        }
    }

    if !verbose {
        info!("");
        info!("Use --verbose for detailed information");
        info!("Use 'jira-db project enable <PROJECT_KEY>' to enable syncing for a project");
    }

    Ok(())
}

async fn handle_config(action: ConfigAction) -> Result<()> {
    let settings_path = Settings::default_path()?;
    let mut settings = Settings::load(&settings_path)?;

    match action {
        ConfigAction::Show => {
            info!("Current configuration:");
            info!("");
            info!("JIRA:");
            info!("  Endpoint: {}", settings.jira.endpoint);
            info!("  Username: {}", settings.jira.username);
            info!("  API Key: {}...", &settings.jira.api_key.chars().take(8).collect::<String>());
            info!("");
            info!("Database:");
            info!("  Path: {}", settings.database.path.display());
            info!("");
            info!("Projects: {} total, {} enabled for sync",
                settings.projects.len(),
                settings.sync_enabled_projects().len()
            );
        }
        ConfigAction::Set { key, value } => {
            match key.as_str() {
                "jira.endpoint" => settings.jira.endpoint = value,
                "jira.username" => settings.jira.username = value,
                "jira.api_key" => settings.jira.api_key = value,
                _ => {
                    return Err(JiraDbError::InvalidConfig(format!(
                        "Unknown configuration key: {}",
                        key
                    )))
                }
            }
            settings.save(&settings_path)?;
            info!("Updated configuration: {}", key);
        }
    }

    Ok(())
}

async fn handle_search(
    query: String,
    _project_filter: Option<String>,
    _status_filter: Option<String>,
) -> Result<()> {
    let settings_path = Settings::default_path()?;
    let settings = Settings::load(&settings_path)?;

    let _db = Database::new(&settings.database.path)?;

    info!("Searching for: {}", query);
    info!("(Search functionality coming soon)");

    Ok(())
}
