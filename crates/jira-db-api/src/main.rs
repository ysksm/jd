//! JiraDb API Server
//!
//! REST API server for JiraDb, providing a web interface to all JiraDb functionality.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use clap::Parser;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod generated;
mod handlers;
mod state;

use generated::create_router;
use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "jira-db-api")]
#[command(about = "JiraDb REST API Server")]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Path to settings.json
    #[arg(long, default_value = "./settings.json")]
    settings: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jira_db_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Initialize application state
    let state = AppState::new(&args.settings)?;

    // Build router
    let app = Router::new()
        .nest("/api", create_router())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(Arc::new(state));

    // Start server
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    tracing::info!("Starting JiraDb API server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
