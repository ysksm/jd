mod application;
mod domain;
mod infrastructure;
mod presentation;

use std::sync::Arc;

use clap::Parser;
use log::error;

use application::services::JiraService;
use domain::error::DomainResult;
use infrastructure::config::Settings;
use infrastructure::database::{
    Database, DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, DuckDbSyncHistoryRepository,
};
use infrastructure::external::jira::JiraApiClient;
use presentation::cli::{Cli, CliHandler, Commands, ConfigAction, ProjectAction};

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
    let settings = Settings::load(&settings_path)?;

    // Create database connection
    let db = Database::new(&settings.database.path)?;
    let conn = db.connection();

    // Create repositories (DIP: these implement domain traits)
    let project_repository = Arc::new(DuckDbProjectRepository::new(conn.clone()));
    let issue_repository = Arc::new(DuckDbIssueRepository::new(conn.clone()));
    let metadata_repository = Arc::new(DuckDbMetadataRepository::new(conn.clone()));
    let change_history_repository = Arc::new(DuckDbChangeHistoryRepository::new(conn.clone()));
    let sync_history_repository = Arc::new(DuckDbSyncHistoryRepository::new(conn.clone()));

    // Create JIRA service (DIP: implements application service trait)
    let jira_service = Arc::new(JiraApiClient::new(&settings.jira)?);

    // Create CLI handler with all dependencies injected
    let handler = CliHandler::new(
        project_repository,
        issue_repository,
        metadata_repository,
        change_history_repository,
        sync_history_repository,
        jira_service,
        settings_path.clone(),
    );

    // Route commands
    match cli.command {
        Commands::Init { .. } => unreachable!(), // Already handled above
        Commands::Project { action } => match action {
            ProjectAction::Init => handler.handle_project_init().await?,
            ProjectAction::List { verbose } => handler.handle_project_list(verbose)?,
            ProjectAction::Enable { project_key } => handler.handle_project_enable(&project_key)?,
            ProjectAction::Disable { project_key } => {
                handler.handle_project_disable(&project_key)?
            }
        },
        Commands::Sync { project, force: _ } => {
            handler.handle_sync(project).await?;
        }
        Commands::Search {
            query,
            project,
            status,
            assignee,
            limit,
            offset,
        } => {
            handler.handle_search(&query, project, status, assignee, limit, offset)?;
        }
        Commands::Metadata { project, r#type } => {
            handler.handle_metadata(&project, r#type)?;
        }
        Commands::History {
            issue_key,
            field,
            limit,
        } => {
            handler.handle_history(&issue_key, field, limit)?;
        }
        Commands::TestTicket {
            project,
            summary,
            description,
            issue_type,
            count,
        } => {
            handler
                .handle_test_ticket(&project, &summary, description.as_deref(), &issue_type, count)
                .await?;
        }
        Commands::Config { action } => match action {
            ConfigAction::Show => handler.handle_config_show()?,
            ConfigAction::Set { key, value } => handler.handle_config_set(&key, &value)?,
        },
    }

    Ok(())
}

async fn handle_init_command(
    settings_path: &std::path::Path,
    interactive: bool,
) -> DomainResult<()> {
    use dialoguer::{Confirm, Input};
    use domain::error::DomainError;
    use log::info;

    if Settings::exists(settings_path) {
        println!(
            "Configuration file already exists at {}",
            settings_path.display()
        );
        return Ok(());
    }

    if interactive {
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
            jira: infrastructure::config::JiraConfig {
                endpoint: endpoint.clone(),
                username: username.clone(),
                api_key: api_key.clone(),
            },
            projects: Vec::new(),
            database: infrastructure::config::DatabaseConfig {
                path: std::path::PathBuf::from(db_path),
            },
        };

        println!("\nTesting JIRA connection...");
        let jira_service = JiraApiClient::new(&settings.jira)?;
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
        info!("  1. Edit the configuration file and set your JIRA credentials:");
        info!("     - endpoint: Your JIRA instance URL");
        info!("     - username: Your JIRA username/email");
        info!("     - api_key: Your JIRA API key");
        info!("  2. Run: jira-db project init");
    }

    Ok(())
}
