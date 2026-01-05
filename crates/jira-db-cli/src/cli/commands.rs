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

    /// Create a test ticket in JIRA for verification
    TestTicket {
        /// Project key to create the test ticket in
        #[arg(short, long)]
        project: String,

        /// Summary/title of the test ticket
        #[arg(short, long, default_value = "[jira-db] 動作確認用テストチケット")]
        summary: String,

        /// Description of the test ticket
        #[arg(short, long)]
        description: Option<String>,

        /// Issue type (e.g., Task, Bug, Story)
        #[arg(short = 't', long, default_value = "Task")]
        issue_type: String,

        /// Number of tickets to create (1-10)
        #[arg(short = 'n', long, default_value = "1")]
        count: usize,
    },

    /// Generate HTML report
    Report {
        /// Project key (or "all" for all enabled projects)
        #[arg(short, long)]
        project: Option<String>,

        /// Generate interactive report with JavaScript
        #[arg(short, long)]
        interactive: bool,

        /// Output file path (default: reports/report_YYYYMMDD_HHMMSS.html)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Generate embeddings for semantic search
    Embeddings {
        /// Project key (or all enabled projects if not specified)
        #[arg(short, long)]
        project: Option<String>,

        /// Regenerate existing embeddings
        #[arg(short, long)]
        force: bool,

        /// Batch size for API calls
        #[arg(short, long, default_value = "50")]
        batch_size: usize,

        /// Embedding provider: openai, ollama, cohere
        #[arg(long)]
        provider: Option<String>,

        /// Model name (provider-specific)
        #[arg(short, long)]
        model: Option<String>,

        /// API endpoint (for Ollama: default http://localhost:11434)
        #[arg(long)]
        endpoint: Option<String>,
    },

    /// Manage issue snapshots (historical versions)
    Snapshots {
        #[command(subcommand)]
        action: SnapshotsAction,
    },

    /// Manage JIRA fields and expand raw data
    Fields {
        #[command(subcommand)]
        action: FieldsAction,
    },

    /// Debug tools for JIRA data creation and testing (requires debug_mode in settings)
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
}

#[derive(Subcommand)]
pub enum SnapshotsAction {
    /// Generate snapshots for a project
    Generate {
        /// Project key
        #[arg(short, long)]
        project: String,
    },

    /// Show snapshots for an issue
    Show {
        /// Issue key (e.g., PROJ-123)
        issue_key: String,

        /// Show specific version
        #[arg(short, long)]
        version: Option<i32>,
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

#[derive(Subcommand)]
pub enum FieldsAction {
    /// Sync field definitions from JIRA
    Sync,

    /// List all stored field definitions
    List {
        /// Show only custom fields
        #[arg(short, long)]
        custom: bool,

        /// Show only navigable fields
        #[arg(short, long)]
        navigable: bool,
    },

    /// Expand raw_data from issues to issues_expanded table
    Expand {
        /// Project key (or all projects if not specified)
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Full sync: fetch fields, add columns, and expand issues
    Full {
        /// Project key (or all projects if not specified)
        #[arg(short, long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum DebugAction {
    /// Create test issues in JIRA
    CreateIssues {
        /// Project key
        #[arg(short, long)]
        project: String,

        /// Number of issues to create (1-100)
        #[arg(short = 'n', long, default_value = "1")]
        count: usize,

        /// Issue type (e.g., Task, Bug, Story)
        #[arg(short = 't', long, default_value = "Task")]
        issue_type: String,

        /// Summary prefix for created issues
        #[arg(short, long, default_value = "[Debug] Test Issue")]
        summary: String,

        /// Description for created issues
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List available status transitions for an issue
    ListTransitions {
        /// Issue key (e.g., PROJ-123)
        issue_key: String,
    },

    /// Transition a single issue to a new status
    TransitionIssue {
        /// Issue key (e.g., PROJ-123)
        issue_key: String,

        /// Transition ID (use list-transitions to find available IDs)
        #[arg(short = 't', long)]
        transition_id: String,
    },

    /// Transition multiple issues to a new status
    BulkTransition {
        /// Issue keys (comma-separated, e.g., PROJ-1,PROJ-2,PROJ-3)
        #[arg(short, long)]
        issues: String,

        /// Transition ID (use list-transitions to find available IDs)
        #[arg(short = 't', long)]
        transition_id: String,
    },

    /// Show current debug mode status
    Status,
}
