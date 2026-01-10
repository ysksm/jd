//! Mock JIRA API Server
//!
//! A mock server that implements the JIRA REST API v3 endpoints used by jira-db.
//! Data is stored in JSON files and can be modified for testing purposes.
//!
//! Usage:
//!   cargo run -p jira-mock-server -- --port 8080 --data-dir ./mock-data
//!
//! The mock server implements the following endpoints:
//! - GET  /rest/api/3/project - List projects
//! - GET  /rest/api/3/search/jql - Search issues (GET)
//! - POST /rest/api/3/search/jql - Search issues (POST)
//! - GET  /rest/api/3/project/{key}/statuses - Get project statuses
//! - GET  /rest/api/3/priority - Get priorities
//! - GET  /rest/api/3/issuetype/project - Get issue types by project ID
//! - GET  /rest/api/3/issue/createmeta/{key}/issuetypes - Get issue types by project key
//! - GET  /rest/api/3/project/{key}/components - Get project components
//! - GET  /rest/api/3/project/{key}/versions - Get project versions
//! - GET  /rest/api/3/field - Get all fields
//! - POST /rest/api/3/issue - Create issue
//! - PUT  /rest/api/3/issue/{key} - Update issue
//! - GET  /rest/api/3/issue/{key}/transitions - Get available transitions
//! - POST /rest/api/3/issue/{key}/transitions - Perform transition
//! - POST /rest/api/3/issueLink - Create issue link

mod data;
mod handlers;

use axum::{
    Router,
    routing::{get, post, put},
};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::data::DataStore;

#[derive(Parser, Debug)]
#[command(name = "jira-mock-server")]
#[command(about = "Mock JIRA API server for testing and development")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Data directory for JSON files
    #[arg(short, long, default_value = "./mock-data")]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jira_mock_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Initialize data store
    let store = Arc::new(DataStore::new(args.data_dir.clone()));
    if let Err(e) = store.load() {
        tracing::error!("Failed to load data: {}", e);
        std::process::exit(1);
    }

    tracing::info!("Data directory: {:?}", args.data_dir);

    // Build router
    let app = Router::new()
        // Projects
        .route("/rest/api/3/project", get(handlers::get_projects))
        // Search
        .route("/rest/api/3/search/jql", get(handlers::search_issues_get))
        .route("/rest/api/3/search/jql", post(handlers::search_issues_post))
        // Legacy search endpoint (some clients use this)
        .route("/rest/api/3/search", get(handlers::search_issues_get))
        .route("/rest/api/3/search", post(handlers::search_issues_post))
        // Project metadata
        .route(
            "/rest/api/3/project/:project_key/statuses",
            get(handlers::get_project_statuses),
        )
        .route("/rest/api/3/priority", get(handlers::get_priorities))
        .route(
            "/rest/api/3/issuetype/project",
            get(handlers::get_issue_types),
        )
        .route(
            "/rest/api/3/issue/createmeta/:project_key/issuetypes",
            get(handlers::get_issue_types_by_project_key),
        )
        .route(
            "/rest/api/3/project/:project_key/components",
            get(handlers::get_components),
        )
        .route(
            "/rest/api/3/project/:project_key/versions",
            get(handlers::get_versions),
        )
        .route("/rest/api/3/field", get(handlers::get_fields))
        // Issue CRUD
        .route("/rest/api/3/issue", post(handlers::create_issue))
        .route("/rest/api/3/issue/:issue_key", put(handlers::update_issue))
        // Transitions
        .route(
            "/rest/api/3/issue/:issue_key/transitions",
            get(handlers::get_transitions),
        )
        .route(
            "/rest/api/3/issue/:issue_key/transitions",
            post(handlers::do_transition),
        )
        // Issue Links
        .route("/rest/api/3/issueLink", post(handlers::create_issue_link))
        // State
        .with_state(store)
        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("🚀 Mock JIRA server listening on http://{}", addr);
    tracing::info!("📁 Data stored in: {:?}", args.data_dir);
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  GET  /rest/api/3/project");
    tracing::info!("  GET  /rest/api/3/search/jql?jql=...");
    tracing::info!("  POST /rest/api/3/search/jql");
    tracing::info!("  GET  /rest/api/3/project/:key/statuses");
    tracing::info!("  GET  /rest/api/3/priority");
    tracing::info!("  GET  /rest/api/3/issuetype/project?projectId=...");
    tracing::info!("  GET  /rest/api/3/issue/createmeta/:key/issuetypes");
    tracing::info!("  GET  /rest/api/3/project/:key/components");
    tracing::info!("  GET  /rest/api/3/project/:key/versions");
    tracing::info!("  GET  /rest/api/3/field");
    tracing::info!("  POST /rest/api/3/issue");
    tracing::info!("  PUT  /rest/api/3/issue/:key");
    tracing::info!("  GET  /rest/api/3/issue/:key/transitions");
    tracing::info!("  POST /rest/api/3/issue/:key/transitions");
    tracing::info!("  POST /rest/api/3/issueLink");

    axum::serve(listener, app).await.unwrap();
}
