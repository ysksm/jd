# jira-db Code Patterns Skill

## Rust Patterns Used

### Error Handling

Use `thiserror` for custom errors:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("JIRA API error: {0}")]
    JiraApi(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
```

### Repository Pattern

Define trait in domain, implement in infrastructure:
```rust
// domain/repositories/issue_repository.rs
pub trait IssueRepository: Send + Sync {
    fn save(&self, issue: &Issue) -> DomainResult<()>;
    fn find_by_key(&self, key: &str) -> DomainResult<Option<Issue>>;
    fn search(&self, params: SearchParams) -> DomainResult<Vec<Issue>>;
}

// infrastructure/database/repositories/issue_repository.rs
pub struct DuckDbIssueRepository {
    conn: DbConnection,
}

impl DuckDbIssueRepository {
    pub fn new(conn: DbConnection) -> Self {
        Self { conn }
    }
}

impl IssueRepository for DuckDbIssueRepository {
    fn save(&self, issue: &Issue) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| DomainError::Database(e.to_string()))?;
        // SQL operations...
    }
}
```

### Use Case Pattern

Encapsulate business operations:
```rust
pub struct SearchIssuesUseCase {
    issue_repo: Arc<dyn IssueRepository>,
}

impl SearchIssuesUseCase {
    pub fn new(issue_repo: Arc<dyn IssueRepository>) -> Self {
        Self { issue_repo }
    }

    pub fn execute(&self, params: SearchParams) -> DomainResult<Vec<Issue>> {
        self.issue_repo.search(params)
    }
}
```

### Dependency Injection

Pass dependencies as Arc<dyn Trait>:
```rust
// In CLI or application setup
let db = Database::new(&settings.database.path)?;
let issue_repo = Arc::new(DuckDbIssueRepository::new(db.connection()));
let use_case = SearchIssuesUseCase::new(issue_repo);

let results = use_case.execute(params)?;
```

### Database Connection Pattern

Use Arc<Mutex<Connection>> for thread-safe access:
```rust
pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    conn: DbConnection,
}

impl Database {
    pub fn new(path: &str) -> DomainResult<Self> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn connection(&self) -> DbConnection {
        Arc::clone(&self.conn)
    }
}
```

### Settings Pattern

JSON-based configuration with defaults:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub jira: JiraConfig,
    pub projects: Vec<ProjectConfig>,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub embeddings: Option<EmbeddingsConfig>,
}

impl Settings {
    pub fn load(path: &Path) -> DomainResult<Self> {
        let content = fs::read_to_string(path)?;
        let settings: Self = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> DomainResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
```

### Entity Pattern

Domain entities are plain structs:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    pub project_key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub created_date: Option<DateTime<Utc>>,
    pub updated_date: Option<DateTime<Utc>>,
    pub raw_data: serde_json::Value,
}
```

### JIRA API Client Pattern

Async HTTP client with authentication:
```rust
pub struct JiraApiClient {
    client: reqwest::Client,
    endpoint: String,
    auth: String,
}

impl JiraApiClient {
    pub fn new(config: &JiraConfig) -> DomainResult<Self> {
        let auth = base64::encode(format!("{}:{}", config.username, config.api_key));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { client, endpoint: config.endpoint.clone(), auth })
    }

    pub async fn search_issues(&self, jql: &str, start_at: u32) -> DomainResult<SearchResponse> {
        let url = format!("{}/rest/api/3/search/jql", self.endpoint);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Basic {}", self.auth))
            .query(&[("jql", jql), ("startAt", &start_at.to_string())])
            .send()
            .await?;
        // Handle response...
    }
}
```

### CLI Command Pattern (Clap)

```rust
#[derive(Parser)]
#[command(name = "jira-db")]
#[command(about = "Sync JIRA data to local DuckDB")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration
    Init {
        #[arg(long)]
        interactive: bool,
    },
    /// Manage projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Sync JIRA data
    Sync {
        #[arg(long)]
        project: Option<String>,
    },
    /// Search issues
    Search {
        query: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}
```

### Progress Bar Pattern

```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(total);
pb.set_style(
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
        .unwrap()
        .progress_chars("#>-"),
);

for item in items {
    // Process item...
    pb.inc(1);
}

pb.finish_with_message("Done");
```

### MCP Tool Pattern

```rust
pub struct SearchIssuesTool {
    issue_repo: Arc<dyn IssueRepository>,
}

impl Tool for SearchIssuesTool {
    fn name(&self) -> &str { "search_issues" }

    fn description(&self) -> &str { "Search JIRA issues" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, ToolError> {
        let query = params["query"].as_str().unwrap_or("");
        // Execute search...
    }
}
```

## Naming Conventions

- **Entities**: PascalCase nouns (`Issue`, `Project`, `ChangeHistory`)
- **Repositories**: `<Entity>Repository` trait, `DuckDb<Entity>Repository` impl
- **Use Cases**: `<Action><Entity>UseCase` (`SyncProjectUseCase`, `SearchIssuesUseCase`)
- **Handlers**: `<action>_handler` functions
- **Configs**: `<Name>Config` structs
