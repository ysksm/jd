# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jira-db` is a command-line tool for synchronizing JIRA data to a local DuckDB database. It enables offline searching and analysis of JIRA issues.

### Key Features
- Sync JIRA projects and issues to a local DuckDB database
- Per-project sync configuration
- Settings managed via `./settings.json` (current directory)
- Full async implementation using Tokio

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
cargo run -- sync              # Sync JIRA data
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
│   ├── client.rs     # Wraps jira-api library
│   └── models.rs     # Domain models (Project, Issue)
├── sync/             # Synchronization logic
│   └── manager.rs    # Orchestrates JIRA to DB sync
├── db/               # Database layer (DuckDB)
│   ├── connection.rs # Connection management
│   ├── schema.rs     # Table definitions
│   └── repository.rs # Data access layer
└── error.rs          # Error types
```

### Database Schema

- **projects**: JIRA project metadata
- **issues**: JIRA issue data with all fields
- **sync_history**: Tracks synchronization operations

All tables include `raw_data` JSON column for complete API responses.

### Dependencies

- **jira-api**: JIRA REST API client (from GitHub)
- **duckdb**: Embedded analytical database
- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **serde/serde_json**: Serialization
- **chrono**: Date/time handling
- **log/env_logger**: Logging

## Usage Workflow

1. **Initialize Config**: `jira-db init` creates `./settings.json` in current directory
2. **Configure**: Edit settings.json with JIRA credentials
3. **Initialize Projects**: `jira-db project init` fetches project list from JIRA
4. **List Projects**: `jira-db project list` shows available projects
5. **Enable Sync**: `jira-db project enable <PROJECT_KEY>` enables sync for specific projects
6. **Sync Data**: `jira-db sync` downloads issues for enabled projects
7. **List Status**: `jira-db project list --verbose` shows detailed sync status

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

- **`init`** - One-time config file creation
- **`project`** - All project management operations
  - `init` - Fetch project list from JIRA
  - `list` - Display projects
  - `enable/disable` - Control sync settings
- **`sync`** - Pure data synchronization
- **`config`** - Global settings management
- **`search`** - Data search (planned)

This design ensures:
- Each command has a single, clear purpose
- Project operations are grouped logically
- `sync` only performs synchronization (requires prior `project init`)

## Implementation Notes

- **Rust Edition**: 2024 (requires Rust 1.85+)
- All JIRA API calls are async using Tokio
- Issue fields are extracted from JSON (`jira_api::Issue.fields` is `serde_json::Value`)
- DuckDB connection is shared via `Arc<Mutex<Connection>>`
- Error handling uses `thiserror` for custom error types
- Sync history tracks all synchronization attempts
- Commands validate prerequisites (e.g., `sync` checks for initialized projects)

## Future Enhancements

- Search functionality (Phase 7 in plan.md)
- Incremental sync (currently only full sync)
- Multiple JIRA instance support
- Export capabilities (CSV, Excel)
- Webhook-based real-time sync
