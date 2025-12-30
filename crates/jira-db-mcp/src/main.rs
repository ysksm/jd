//! MCP Server for JIRA Database
//!
//! This binary provides an MCP (Model Context Protocol) server that allows
//! AI assistants to query JIRA data stored in a local DuckDB database.

mod config;
mod handlers;
mod protocol;
mod server;
mod tools;
mod transport;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use config::McpConfig;
use server::McpServer;

/// MCP Server for JIRA Database queries
#[derive(Parser, Debug)]
#[command(name = "jira-db-mcp")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable HTTP server mode instead of stdio
    #[arg(long)]
    http: bool,

    /// Port for HTTP server (default: 3000)
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Host for HTTP server (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Initialize a new configuration file
    #[arg(long)]
    init: bool,

    /// Database path (overrides config file)
    #[arg(long)]
    database: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (important for MCP - stdout is for protocol)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("jira_db_mcp=info".parse()?))
        .init();

    let args = Args::parse();

    // Handle init command
    if args.init {
        let config_path = args.config.unwrap_or_else(McpConfig::default_path);
        if config_path.exists() {
            anyhow::bail!(
                "Configuration file already exists: {}",
                config_path.display()
            );
        }
        let config = McpConfig::default_config();
        config.save(&config_path)?;
        eprintln!("Created configuration file: {}", config_path.display());
        return Ok(());
    }

    // Load or create configuration
    let config_path = args.config.unwrap_or_else(McpConfig::default_path);
    let mut config = if config_path.exists() {
        McpConfig::load(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        tracing::warn!(
            "No configuration file found at {}. Using default configuration.",
            config_path.display()
        );
        McpConfig::default_config()
    };

    // Override database path if provided
    if let Some(db_path) = args.database {
        config.database_path = db_path;
    }

    // Create and run server
    let server = McpServer::new(config).context("Failed to create MCP server")?;

    if args.http {
        server.run_http(&args.host, args.port).await
    } else {
        server.run_stdio().await
    }
}
