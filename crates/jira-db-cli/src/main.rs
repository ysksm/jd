mod cli;

use std::sync::Arc;

use clap::Parser;
use log::error;

use jira_db_core::application::services::JiraService;
use jira_db_core::domain::error::DomainResult;
use jira_db_core::infrastructure::config::Settings;
use jira_db_core::infrastructure::database::{
    Database, DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbIssueSnapshotRepository,
    DuckDbMetadataRepository, DuckDbProjectRepository, DuckDbSyncHistoryRepository,
};
use jira_db_core::infrastructure::external::jira::JiraApiClient;

use cli::{Cli, CliHandler, Commands, ConfigAction, FieldsAction, ProjectAction, SnapshotsAction};

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
    let snapshot_repository = Arc::new(DuckDbIssueSnapshotRepository::new(conn.clone()));

    // Create JIRA service (DIP: implements application service trait)
    let jira_service = Arc::new(JiraApiClient::new(&settings.jira)?);

    // Clone repositories for later use in embeddings and fields commands
    let issue_repository_for_embeddings = issue_repository.clone();
    let jira_service_for_fields = Arc::clone(&jira_service);

    // Create CLI handler with all dependencies injected
    let handler = CliHandler::new(
        project_repository,
        issue_repository,
        metadata_repository,
        change_history_repository,
        sync_history_repository,
        snapshot_repository,
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
                .handle_test_ticket(
                    &project,
                    &summary,
                    description.as_deref(),
                    &issue_type,
                    count,
                )
                .await?;
        }
        Commands::Config { action } => match action {
            ConfigAction::Show => handler.handle_config_show()?,
            ConfigAction::Set { key, value } => handler.handle_config_set(&key, &value)?,
        },
        Commands::Report {
            project,
            interactive,
            output,
        } => {
            handler.handle_report(project, interactive, output)?;
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
                &settings,
                conn.clone(),
                issue_repository_for_embeddings,
                project,
                force,
                batch_size,
                provider,
                model,
                endpoint,
            )
            .await?;
        }
        Commands::Snapshots { action } => match action {
            SnapshotsAction::Generate { project } => {
                handler.handle_snapshots_generate(&project)?;
            }
            SnapshotsAction::Show { issue_key, version } => {
                handler.handle_snapshots_show(&issue_key, version)?;
            }
        },
        Commands::Fields { action } => {
            handle_fields_command(conn.clone(), jira_service_for_fields, action).await?;
        }
    }

    Ok(())
}

async fn handle_init_command(
    settings_path: &std::path::Path,
    interactive: bool,
) -> DomainResult<()> {
    use dialoguer::{Confirm, Input};
    use jira_db_core::domain::error::DomainError;
    use jira_db_core::infrastructure::config::{DatabaseConfig, JiraConfig};
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
            jira: JiraConfig {
                endpoint: endpoint.clone(),
                username: username.clone(),
                api_key: api_key.clone(),
            },
            projects: Vec::new(),
            database: DatabaseConfig {
                path: std::path::PathBuf::from(db_path),
            },
            embeddings: None,
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

async fn handle_embeddings_command(
    settings: &Settings,
    conn: jira_db_core::infrastructure::database::DbConnection,
    issue_repository: Arc<DuckDbIssueRepository>,
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

    // Create embeddings repository
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

    let result = use_case.execute(project.as_deref()).await?;

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

async fn handle_fields_command(
    conn: jira_db_core::infrastructure::database::DbConnection,
    jira_service: Arc<JiraApiClient>,
    action: FieldsAction,
) -> DomainResult<()> {
    use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
    use jira_db_core::application::use_cases::SyncFieldsUseCase;
    use jira_db_core::infrastructure::database::{
        DuckDbFieldRepository, DuckDbIssuesExpandedRepository,
    };

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

        FieldsAction::Expand { project } => {
            println!("Adding columns based on field definitions...");
            let use_case = SyncFieldsUseCase::new(jira_service, field_repo, expanded_repo.clone());

            let added = use_case.add_columns()?;
            if !added.is_empty() {
                println!("Added {} new columns: {}", added.len(), added.join(", "));
            }

            println!("Expanding issues from raw_data...");
            let count = use_case.expand_issues(project.as_deref())?;
            println!("Expanded {} issues into issues_expanded table", count);
        }

        FieldsAction::Full { project } => {
            println!("Running full field sync and expansion...\n");
            let use_case = SyncFieldsUseCase::new(jira_service, field_repo, expanded_repo);

            let result = use_case.execute(project.as_deref()).await?;

            println!("Results:");
            println!("  Fields synced:    {}", result.fields_synced);
            println!("  Columns added:    {}", result.columns_added);
            println!("  Issues expanded:  {}", result.issues_expanded);
        }
    }

    Ok(())
}
