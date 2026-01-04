//! JiraDb Web Server
//!
//! A web application server using ActixWeb + Angular.

use std::path::PathBuf;
use std::sync::Arc;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{App, HttpServer, middleware, web};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;

use config::Config;
use jira_db_service::AppState;

#[derive(Parser, Debug)]
#[command(name = "jira-db-web")]
#[command(about = "JiraDb Web Application Server")]
struct Args {
    /// Path to config file (TOML)
    #[arg(short, long, default_value = "./config.toml")]
    config: String,

    /// Host to bind to (overrides config file)
    #[arg(long)]
    host: Option<String>,

    /// Port to bind to (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,

    /// Path to settings.json (overrides config file)
    #[arg(long)]
    settings: Option<String>,

    /// Path to static files (overrides config file)
    #[arg(long)]
    static_dir: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jira_db_web=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Load config from file
    let config = Config::load_or_default(&args.config);
    tracing::info!("Loaded config from {}", args.config);

    // Apply CLI overrides
    let host = args.host.unwrap_or(config.server.host);
    let port = args.port.unwrap_or(config.server.port);
    let settings_path_str = args.settings.unwrap_or(config.app.settings_path);
    let static_dir = args.static_dir.unwrap_or(config.app.static_dir);

    // Initialize application state
    let state = Arc::new(AppState::new());

    // Initialize with settings file if it exists
    let settings_path = PathBuf::from(&settings_path_str);
    if settings_path.exists() {
        if let Err(e) = state.initialize(settings_path.clone()) {
            tracing::warn!("Failed to load settings from {}: {}", settings_path_str, e);
            tracing::info!(
                "Server will start without initialized state. Use /api/config.initialize to configure."
            );
        } else {
            tracing::info!("Loaded settings from {}", settings_path_str);
        }
    } else {
        tracing::info!(
            "Settings file not found at {}. Use /api/config.initialize to configure.",
            settings_path_str
        );
        // Store the settings path for later use
        *state.settings_path.lock().unwrap() = Some(settings_path);
    }

    let bind_addr = format!("{}:{}", host, port);

    tracing::info!("Starting JiraDb Web Server on http://{}", bind_addr);
    tracing::info!("Static files directory: {}", static_dir);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let mut app = App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(state.clone()))
            // API routes
            .service(
                web::scope("/api")
                    // Config
                    .route("/config.get", web::post().to(handlers::config_get))
                    .route("/config.update", web::post().to(handlers::config_update))
                    .route(
                        "/config.initialize",
                        web::post().to(handlers::config_initialize),
                    )
                    // Projects
                    .route("/projects.list", web::post().to(handlers::projects_list))
                    .route(
                        "/projects.initialize",
                        web::post().to(handlers::projects_initialize),
                    )
                    .route(
                        "/projects.enable",
                        web::post().to(handlers::projects_enable),
                    )
                    .route(
                        "/projects.disable",
                        web::post().to(handlers::projects_disable),
                    )
                    // Sync
                    .route("/sync.execute", web::post().to(handlers::sync_execute))
                    .route("/sync.status", web::post().to(handlers::sync_status))
                    // Issues
                    .route("/issues.search", web::post().to(handlers::issues_search))
                    .route("/issues.get", web::post().to(handlers::issues_get))
                    .route("/issues.history", web::post().to(handlers::issues_history))
                    // Metadata
                    .route("/metadata.get", web::post().to(handlers::metadata_get))
                    // Embeddings
                    .route(
                        "/embeddings.generate",
                        web::post().to(handlers::embeddings_generate),
                    )
                    .route(
                        "/embeddings.search",
                        web::post().to(handlers::embeddings_search),
                    )
                    // Reports
                    .route(
                        "/reports.generate",
                        web::post().to(handlers::reports_generate),
                    )
                    // SQL
                    .route("/sql.execute", web::post().to(handlers::sql_execute))
                    .route("/sql.get-schema", web::post().to(handlers::sql_get_schema))
                    .route(
                        "/sql.list-queries",
                        web::post().to(handlers::sql_query_list),
                    )
                    .route("/sql.save-query", web::post().to(handlers::sql_query_save))
                    .route(
                        "/sql.delete-query",
                        web::post().to(handlers::sql_query_delete),
                    ),
            );

        // Serve static files if directory exists
        let static_path = PathBuf::from(&static_dir);
        if static_path.exists() {
            app = app.service(
                Files::new("/", &static_dir)
                    .index_file("index.html")
                    .default_handler(
                        actix_files::NamedFile::open(static_path.join("index.html"))
                            .unwrap_or_else(|_| {
                                // Fallback: create an empty file handler
                                panic!("index.html not found in static directory")
                            }),
                    ),
            );
        }

        app
    })
    .bind(&bind_addr)?
    .run()
    .await
}
