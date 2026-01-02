# jira-db Architecture Skill

## Overview

jira-db is a Rust workspace project for synchronizing JIRA data to a local DuckDB database with MCP server support for AI integration.

## Workspace Structure

```
jira-db/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── jira-db-core/             # Core library (domain, application, infrastructure)
│   ├── jira-db-cli/              # CLI binary (jira-db command)
│   ├── jira-db-mcp/              # MCP Server (stdio + HTTP)
│   ├── jira-db-tauri/            # Tauri GUI application
│   └── jira-db-api/              # REST API server
```

## Layer Architecture (jira-db-core)

The core library follows Clean Architecture / Hexagonal Architecture:

### 1. Domain Layer (`domain/`)
Pure business logic with no I/O dependencies:
- `entities/` - Business entities: Project, Issue, ChangeHistory, Metadata types
- `repositories/` - Abstract repository interfaces (traits)
- `error.rs` - DomainError, DomainResult types

### 2. Application Layer (`application/`)
Use cases orchestrating domain + infrastructure:
- `use_cases/` - Business operations: SyncProject, SearchIssues, GenerateEmbeddings, etc.
- `services/` - JiraService trait for JIRA operations
- `dto/` - Data Transfer Objects: SyncResult, CreatedIssueDto

### 3. Infrastructure Layer (`infrastructure/`)
External integrations and implementations:
- `config/` - Settings management (JSON-based configuration)
- `database/` - DuckDB repositories implementing domain traits
- `external/jira/` - JIRA REST API v3 client
- `external/embeddings/` - Embedding providers (OpenAI, Ollama, Cohere)

### 4. Report Layer (`report/`)
HTML report generation:
- `static_report.rs` - Static HTML reports
- `interactive/` - JavaScript-based dashboard

## Key Design Patterns

### Repository Pattern
Domain defines repository traits, infrastructure implements them:
```rust
// domain/repositories/issue_repository.rs
pub trait IssueRepository {
    fn save(&self, issue: &Issue) -> DomainResult<()>;
    fn find_by_key(&self, key: &str) -> DomainResult<Option<Issue>>;
}

// infrastructure/database/repositories/issue_repository.rs
pub struct DuckDbIssueRepository { ... }
impl IssueRepository for DuckDbIssueRepository { ... }
```

### Use Case Pattern
Each business operation is encapsulated in a use case:
```rust
pub struct SyncProjectUseCase {
    project_repo: Arc<dyn ProjectRepository>,
    issue_repo: Arc<dyn IssueRepository>,
    jira_service: Arc<dyn JiraService>,
}
```

### Dependency Injection
Use cases receive dependencies through constructor:
```rust
let use_case = SearchIssuesUseCase::new(Arc::new(repo));
let issues = use_case.execute(SearchParams { ... })?;
```

## Database Schema

### Core Tables
- `projects` - JIRA project metadata
- `issues` - JIRA issues with `raw_data` JSON containing full API response
- `sync_history` - Sync operation tracking
- `issue_change_history` - Normalized changelog

### Metadata Tables
- `statuses`, `priorities`, `issue_types`, `labels`, `components`, `fix_versions`
- All use composite primary key `(project_id, name)`

### Embeddings Tables
- `issue_embeddings` - Vector embeddings for semantic search
- Uses DuckDB VSS extension with HNSW index

## MCP Server Architecture (jira-db-mcp)

### Transports
- **Stdio**: For Claude Desktop, VS Code integration
- **HTTP/SSE**: For web clients

### Protocol
- JSON-RPC 2.0 based
- MCP (Model Context Protocol) specification

### Tools
Read-only tools for AI assistants:
- `search_issues` - Full-text search with filters
- `get_issue` - Issue details by key
- `semantic_search` - Vector similarity search
- `execute_sql` - Read-only SQL execution

## Configuration

Settings stored in `./settings.json` (current directory):
```json
{
  "jira": { "endpoint": "...", "username": "...", "api_key": "..." },
  "projects": [...],
  "database": { "path": "./data/jira.duckdb" },
  "embeddings": { "provider": "ollama", "model": "nomic-embed-text" }
}
```

## Error Handling

- `DomainError` - Business logic errors
- `thiserror` for custom error types
- `Result<T, DomainError>` aliased as `DomainResult<T>`

## Async Runtime

- Tokio for async operations
- JIRA API calls are async
- DuckDB operations are synchronous (wrapped in blocking tasks when needed)
