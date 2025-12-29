# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jira-db` is a command-line tool for synchronizing JIRA data to a local DuckDB database. It enables offline searching and analysis of JIRA issues.

### Key Features
- Sync JIRA projects and issues to a local DuckDB database
- Per-project sync configuration
- Settings managed via `./settings.json` (current directory)
- Full async implementation using Tokio
- Metadata management (statuses, priorities, issue types, labels, components, versions)
- Complete issue data capture (all fields + changelog)
- Direct JIRA REST API v3 integration with reqwest

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

### Running
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

The project follows a modular architecture:

```
src/
├── main.rs           # Entry point and CLI command handlers
├── cli/              # CLI command definitions (using clap)
├── config/           # Settings management (JSON-based config)
├── jira/             # JIRA API client wrapper
│   ├── client.rs     # Direct JIRA REST API v3 calls with reqwest
│   └── models.rs     # Domain models (Project, Issue, Status, Priority, etc.)
├── sync/             # Synchronization logic
│   └── manager.rs    # Orchestrates JIRA to DB sync (issues + metadata)
├── db/               # Database layer (DuckDB)
│   ├── connection.rs # Connection management
│   ├── schema.rs     # Table definitions (issues, metadata tables)
│   └── repository.rs # Data access layer (IssueRepository, MetadataRepository)
└── error.rs          # Error types
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

1. **Initialize Config**: `jira-db init` creates `./settings.json` in current directory
2. **Configure**: Edit settings.json with JIRA credentials
3. **Initialize Projects**: `jira-db project init` fetches project list from JIRA
4. **List Projects**: `jira-db project list` shows available projects
5. **Enable Sync**: `jira-db project enable <PROJECT_KEY>` enables sync for specific projects
6. **Sync Data**: `jira-db sync` downloads issues + metadata for enabled projects
7. **Search Issues**: `jira-db search <QUERY>` searches synced issues
8. **View Metadata**: `jira-db metadata --project <KEY>` shows project metadata
9. **View Change History**: `jira-db history <ISSUE_KEY>` shows issue change history
10. **List Status**: `jira-db project list --verbose` shows detailed sync status

## Configuration

Located at `./settings.json` (current directory):

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

- ✅ Search functionality with full-text and filtering
- ✅ Metadata management (statuses, priorities, issue types, labels, components, versions)
- ✅ Complete issue data capture (all fields + changelog)
- ✅ Change history management with dedicated table and CLI command
- ✅ Interactive initialization
- ✅ Progress bars for sync operations
- ✅ Comprehensive error handling

## Future Enhancements

- Incremental sync (currently only full sync)
- Multiple JIRA instance support
- Export capabilities (CSV, Excel)
- Webhook-based real-time sync
- Unit and integration tests
