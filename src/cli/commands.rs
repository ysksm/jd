use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jira-db")]
#[command(about = "JIRA data synchronization and local database tool", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration file
    Init {
        /// Interactive configuration setup
        #[arg(short, long)]
        interactive: bool,
    },

    /// Manage JIRA projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Synchronize JIRA data for enabled projects
    Sync {
        /// Specific project key to sync (syncs all enabled projects if not specified)
        #[arg(short, long)]
        project: Option<String>,

        /// Force full synchronization
        #[arg(short, long)]
        force: bool,
    },

    /// Configure settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Search issues
    Search {
        /// Search query
        query: String,

        /// Filter by project key
        #[arg(short, long)]
        project: Option<String>,

        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by assignee
        #[arg(short, long)]
        assignee: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },

    /// List metadata (statuses, priorities, etc.)
    Metadata {
        /// Project key to show metadata for
        #[arg(short, long)]
        project: String,

        /// Type of metadata to show (status, priority, issue-type, label, component, version)
        #[arg(short, long)]
        r#type: Option<String>,
    },

    /// Show change history for an issue
    History {
        /// Issue key (e.g., PROJ-123)
        issue_key: String,

        /// Filter by field name (e.g., status, assignee, priority)
        #[arg(short, long)]
        field: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// Initialize project list from JIRA
    Init,

    /// List all projects
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Enable sync for a project
    Enable {
        /// Project key
        project_key: String,
    },

    /// Disable sync for a project
    Disable {
        /// Project key
        project_key: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set configuration value
    Set {
        /// Configuration key (e.g., jira.endpoint)
        key: String,

        /// Configuration value
        value: String,
    },
}
