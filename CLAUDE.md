# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jira-db` is a command-line tool for synchronizing JIRA data to a local DuckDB database. It enables offline searching and analysis of JIRA issues.

### Key Features
- Sync JIRA projects and issues to a local DuckDB database
- Per-project sync configuration
- Settings managed via `./data/settings.json`
- Full async implementation using Tokio
- Metadata management (statuses, priorities, issue types, labels, components, versions)
- Complete issue data capture (all fields + changelog)
- Direct JIRA REST API v3 integration with reqwest
- **MCP Server**: Model Context Protocol server for AI integration (stdio + HTTP)
- **Vector Search**: Semantic search using DuckDB VSS with multiple embedding providers (OpenAI, Ollama, Cohere)

## Prerequisites

### System Dependencies
DuckDB must be installed on your system:

```bash
# macOS
brew install duckdb

# The following environment variables may be needed for building:
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
```

## Development Commands

### Building
```bash
# Standard build
cargo build

# Release build (with library path for macOS)
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH" && cargo build --release
```

### Running CLI
```bash
cargo run -- init              # Initialize configuration file
cargo run -- project init      # Initialize project list from JIRA
cargo run -- project list      # List projects
cargo run -- project enable PROJ  # Enable sync for a project
cargo run -- sync              # Sync JIRA data (issues + metadata)
cargo run -- search "bug"      # Search issues
cargo run -- metadata --project PROJ  # View project metadata
cargo run -- history PROJ-123  # View change history for an issue
cargo run -- config show       # Show configuration
cargo run -- embeddings        # Generate embeddings for semantic search
cargo run -- report --interactive  # Generate interactive HTML report
```

### Running MCP Server
```bash
# Stdio mode (for Claude Desktop, VS Code, etc.)
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb

# HTTP mode (for web clients)
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb --http --port 8080
```

### Testing
```bash
cargo test                     # Run all tests
cargo test <test_name>         # Run a specific test
cargo test -- --nocapture      # Run tests with output
```

### Code Quality
```bash
cargo check          # Quick compile check
cargo clippy         # Run linter
cargo fmt            # Format code
```

## Architecture

The project is organized as a Cargo workspace with three crates:

```
jira-db/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── jira-db-core/             # Core library (reusable by GUI/MCP)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Public API exports
│   │       ├── domain/           # Business entities & repository traits
│   │       │   ├── entities/     # Project, Issue, ChangeHistory, Metadata
│   │       │   ├── repositories/ # Abstract repository interfaces
│   │       │   └── error.rs      # DomainError, DomainResult
│   │       ├── application/      # Use cases & services
│   │       │   ├── use_cases/    # SyncProject, SearchIssues, GenerateEmbeddings, etc.
│   │       │   ├── services/     # JiraService trait
│   │       │   └── dto/          # SyncResult, CreatedIssueDto
│   │       ├── infrastructure/   # External integrations
│   │       │   ├── config/       # Settings (JSON-based)
│   │       │   ├── database/     # DuckDB repositories + EmbeddingsRepository
│   │       │   └── external/     # JIRA API client + OpenAI embeddings
│   │       └── report/           # HTML report generation
│   │           ├── static_report.rs
│   │           └── interactive/  # JavaScript-based dashboard
│   ├── jira-db-cli/              # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       └── cli/              # Command handlers (clap)
│   │           ├── commands.rs   # CLI command definitions
│   │           └── handlers.rs   # Command implementations
│   └── jira-db-mcp/              # MCP Server binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # Entry point
│           ├── config.rs         # Server configuration
│           ├── server.rs         # MCP server logic
│           ├── protocol/         # JSON-RPC 2.0 + MCP types
│           ├── handlers/         # Request handlers
│           ├── tools/            # Tool definitions & implementations
│           └── transport/        # Stdio + HTTP transports
```

### Crate Responsibilities

#### jira-db-core (Library)
- **domain**: Pure business logic, no I/O dependencies
- **application**: Use cases orchestrating domain + infrastructure
- **infrastructure**: DuckDB, JIRA API, config file handling
- **report**: HTML report generation (static & interactive)

#### jira-db-cli (Binary)
- CLI-specific code (clap, dialoguer, comfy-table)
- Command routing and user interaction
- Binary name: `jira-db`

#### jira-db-mcp (Binary)
- MCP (Model Context Protocol) server for AI integration
- JSON-RPC 2.0 based communication
- Supports stdio (Claude Desktop) and HTTP (web clients) transports
- Read-only tools for issue search, metadata access, SQL execution
- Binary name: `jira-db-mcp`

### MCP Server Tools

The MCP server provides these tools for AI assistants:

| Tool | Description |
|------|-------------|
| `search_issues` | Full-text search with filters (project, status, assignee) |
| `get_issue` | Get issue details by key |
| `get_issue_history` | Get change history for an issue |
| `list_projects` | List all synced projects |
| `get_project_metadata` | Get project metadata (statuses, priorities, etc.) |
| `get_schema` | Get database schema for SQL queries |
| `execute_sql` | Execute read-only SQL queries |
| `semantic_search` | Vector similarity search using embeddings |

### Vector Search (Embeddings)

The system supports semantic search using DuckDB VSS extension with multiple embedding providers:

#### Supported Providers

| Provider | Environment Variable | Default Model | Dimensions |
|----------|---------------------|---------------|------------|
| OpenAI | `OPENAI_API_KEY` | text-embedding-3-small | 1536 |
| Ollama | - (local) | nomic-embed-text | 768 |
| Cohere | `COHERE_API_KEY` | embed-multilingual-v3.0 | 1024 |

#### Usage

```bash
# Ollama (free, local)
jira-db embeddings --provider ollama

# OpenAI
jira-db embeddings --project PROJ

# Cohere (good for Japanese)
jira-db embeddings --provider cohere
```

#### Configuration

```json
{
  "embeddings": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "endpoint": "http://localhost:11434",
    "auto_generate": false
  }
}
```

#### Embedding Details
- Distance: Cosine similarity
- Index: HNSW (Hierarchical Navigable Small World)
- Text: Concatenated issue fields (key, summary, description, status, priority, etc.)

See [docs/EMBEDDINGS.md](docs/EMBEDDINGS.md) for detailed provider documentation.

### Using Core in Future GUI

```rust
use jira_db_core::{
    Settings, Database, JiraApiClient,
    DuckDbIssueRepository, SearchIssuesUseCase, SearchParams,
};

// Initialize
let settings = Settings::load(&path)?;
let db = Database::new(&settings.database.path)?;
let repo = DuckDbIssueRepository::new(db.connection());

// Execute use case
let use_case = SearchIssuesUseCase::new(Arc::new(repo));
let issues = use_case.execute(SearchParams { query: Some("bug".into()), ..Default::default() })?;
```

### Database Schema

#### Core Tables
- **projects**: JIRA project metadata
- **issues**: JIRA issue data with all fields (includes `raw_data` JSON with full API response + changelog)
- **sync_history**: Tracks synchronization operations
- **issue_change_history**: Normalized change history extracted from issue changelog

#### Change History Table
The `issue_change_history` table stores normalized change history data:
- `issue_id`, `issue_key`: Issue identifiers
- `history_id`: JIRA history entry ID
- `author_account_id`, `author_display_name`: Who made the change
- `field`: Field that was changed (e.g., status, assignee, priority)
- `field_type`: Type of field
- `from_value`, `from_string`: Previous value
- `to_value`, `to_string`: New value
- `changed_at`: When the change occurred

#### Metadata Tables
- **statuses**: Project status definitions (name, description, category)
- **priorities**: Priority definitions (name, description, icon_url)
- **issue_types**: Issue type definitions (name, description, icon_url, subtask flag)
- **labels**: Project labels
- **components**: Component definitions (name, description, lead)
- **fix_versions**: Version/release definitions (name, description, released flag, release_date)

All metadata tables use composite primary key `(project_id, name)` for uniqueness per project.

### Dependencies

- **jira-api**: JIRA REST API client (from GitHub) - used for project fetching
- **reqwest**: HTTP client for direct JIRA REST API v3 calls (issue + metadata fetching)
- **base64**: Base64 encoding for HTTP Basic Authentication
- **duckdb**: Embedded analytical database
- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **serde/serde_json**: Serialization
- **chrono**: Date/time handling
- **log/env_logger**: Logging
- **indicatif**: Progress bars for sync operations
- **dialoguer**: Interactive prompts for init
- **comfy-table**: Table formatting for CLI output

## Usage Workflow

1. **Initialize Config**: `jira-db init` creates `./data/settings.json`
2. **Configure**: Edit `./data/settings.json` with JIRA credentials
3. **Initialize Projects**: `jira-db project init` fetches project list from JIRA
4. **List Projects**: `jira-db project list` shows available projects
5. **Enable Sync**: `jira-db project enable <PROJECT_KEY>` enables sync for specific projects
6. **Sync Data**: `jira-db sync` downloads issues + metadata for enabled projects
7. **Search Issues**: `jira-db search <QUERY>` searches synced issues
8. **View Metadata**: `jira-db metadata --project <KEY>` shows project metadata
9. **View Change History**: `jira-db history <ISSUE_KEY>` shows issue change history
10. **List Status**: `jira-db project list --verbose` shows detailed sync status

## Configuration

Located at `./data/settings.json`:

```json
{
  "jira": {
    "endpoint": "https://your-domain.atlassian.net",
    "username": "user@example.com",
    "api_key": "your-api-key"
  },
  "projects": [],
  "database": {
    "path": "./data/jira.duckdb"
  }
}
```

## CLI Structure

The CLI is organized with clear separation of concerns:

- **`init`** - One-time config file creation (supports `--interactive` mode)
- **`project`** - All project management operations
  - `init` - Fetch project list from JIRA
  - `list` - Display projects (supports `--verbose`)
  - `enable/disable` - Control sync settings
- **`sync`** - Pure data synchronization (issues + metadata)
- **`config`** - Global settings management
  - `show` - Display current configuration
  - `set` - Update configuration values
- **`search`** - Full-text search with filtering
  - Supports project, status, assignee filters
  - Pagination with `--limit` and `--offset`
- **`metadata`** - View project metadata
  - Supports filtering by type (status, priority, issue-type, label, component, version)
- **`history`** - View change history for an issue
  - Shows all field changes with timestamps and authors
  - Supports filtering by field name (`--field status`)
  - Pagination with `--limit`
- **`embeddings`** - Generate embeddings for semantic search
  - Requires OpenAI API key (env or settings.json)
  - Supports `--project`, `--force`, `--batch-size` options
  - Displays timing breakdown
- **`report`** - Generate HTML reports
  - Supports `--interactive` for JavaScript dashboard
  - Custom output with `--output`

This design ensures:
- Each command has a single, clear purpose
- Project operations are grouped logically
- `sync` performs both issue and metadata synchronization
- Metadata is fetched directly from JIRA API, not extracted from issues

## Implementation Notes

- **Rust Edition**: 2024 (requires Rust 1.85+)
- All JIRA API calls are async using Tokio
- DuckDB connection is shared via `Arc<Mutex<Connection>>`
- Error handling uses `thiserror` for custom error types
- Sync history tracks all synchronization attempts
- Commands validate prerequisites (e.g., `sync` checks for initialized projects)

### JIRA API Integration

#### Issue Fetching
- Uses direct JIRA REST API v3 endpoint: `/rest/api/3/search/jql` (NOT `/rest/api/3/search` - deprecated)
- Query parameters:
  - `fields=*navigable` - Fetches all navigable fields (NOT `*all` - causes 410 error)
  - `expand=changelog` - Includes complete change history
  - `maxResults=100` - Pagination size
- Complete JSON response (including changelog) stored in `issues.raw_data`
- Manual pagination implementation to fetch all issues

#### Metadata Fetching
Metadata is fetched from dedicated JIRA API endpoints:
- **Statuses**: `GET /rest/api/3/project/{projectKey}/statuses`
- **Priorities**: `GET /rest/api/3/priority`
- **Issue Types**: `GET /rest/api/3/issuetype/project?projectId={projectId}`
- **Labels**: `GET /rest/api/3/search/jql?jql=project={key} AND labels is not EMPTY`
- **Components**: `GET /rest/api/3/project/{projectKey}/components`
- **Versions**: `GET /rest/api/3/project/{projectKey}/versions`

Metadata is NOT extracted from issues - it's fetched from API to ensure completeness.

#### Authentication
- HTTP Basic Authentication using base64-encoded credentials
- Authorization header: `Basic base64(username:api_token)`

### Database Operations
- Metadata tables use `ON CONFLICT DO UPDATE` for upsert operations
- All metadata includes `created_at` and `updated_at` timestamps
- Composite primary keys `(project_id, name)` ensure uniqueness per project

## Completed Features

- ✅ Core/CLI workspace split for GUI support
- ✅ Search functionality with full-text and filtering
- ✅ Metadata management (statuses, priorities, issue types, labels, components, versions)
- ✅ Complete issue data capture (all fields + changelog)
- ✅ Change history management with dedicated table and CLI command
- ✅ Interactive initialization
- ✅ Progress bars for sync operations
- ✅ Comprehensive error handling
- ✅ HTML reports (static & interactive dashboard)
- ✅ **MCP Server** with stdio and HTTP/SSE transports
- ✅ **Vector Search** with DuckDB VSS and OpenAI embeddings
- ✅ **Semantic Search** tool via MCP
- ✅ **SQL Execution** tool via MCP (read-only)
- ✅ **Multiple Embedding Providers** (OpenAI, Ollama, Cohere)

## Future Enhancements

- GUI implementation (Tauri desktop app or Web server)
- Incremental sync (currently only full sync)
- Multiple JIRA instance support
- Export capabilities (CSV, Excel)
- Webhook-based real-time sync
- Automatic embedding generation during sync
- Azure OpenAI / Voyage AI embedding providers
