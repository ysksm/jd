# jira-db Development Skill

## Development Environment

### Prerequisites
- Rust 1.85+ (Edition 2024)
- DuckDB installed on system

### macOS Setup
```bash
brew install duckdb
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
```

## Build Commands

```bash
# Development build
cargo build

# Release build (with library path for macOS)
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH" && cargo build --release

# Build specific crate
cargo build -p jira-db-core
cargo build -p jira-db-cli
cargo build -p jira-db-mcp
cargo build -p jira-db-tauri
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test <test_name>

# Run tests with output
cargo test -- --nocapture

# Run tests for specific crate
cargo test -p jira-db-core
```

## Code Quality

```bash
# Quick compile check
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Check formatting without changing
cargo fmt --check
```

## Running the CLI

```bash
# Run CLI commands
cargo run -p jira-db-cli -- init
cargo run -p jira-db-cli -- project list
cargo run -p jira-db-cli -- sync
cargo run -p jira-db-cli -- search "query"

# Or after building
./target/debug/jira-db <command>
```

## Running MCP Server

```bash
# Stdio mode (for Claude Desktop)
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb

# HTTP mode
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb --http --port 8080
```

## Running Tauri App

Tauri uses Angular for the frontend.

```bash
# Navigate to Tauri crate
cd crates/jira-db-tauri

# Install npm dependencies (first time only)
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## Adding Dependencies

Dependencies should be added to the appropriate crate:
- `jira-db-core` - Core functionality shared by all binaries
- `jira-db-cli` - CLI-specific dependencies (clap, dialoguer, comfy-table)
- `jira-db-mcp` - MCP-specific dependencies
- `jira-db-tauri` - Tauri-specific dependencies

## Code Organization Guidelines

### When adding new functionality:

1. **Domain entities** go in `jira-db-core/src/domain/entities/`
2. **Repository traits** go in `jira-db-core/src/domain/repositories/`
3. **Use cases** go in `jira-db-core/src/application/use_cases/`
4. **Database implementations** go in `jira-db-core/src/infrastructure/database/`
5. **External API clients** go in `jira-db-core/src/infrastructure/external/`

### When adding CLI commands:

1. Add command definition in `jira-db-cli/src/cli/commands.rs`
2. Add handler in `jira-db-cli/src/cli/handlers.rs`
3. Use cases should be in core, not CLI

### When adding MCP tools:

1. Define tool in `jira-db-mcp/src/tools/`
2. Register handler in `jira-db-mcp/src/handlers/`

## Common Development Tasks

### Adding a new entity
1. Create entity in `domain/entities/`
2. Add to `domain/entities/mod.rs` exports
3. Re-export in `lib.rs` if needed externally

### Adding a new use case
1. Create use case in `application/use_cases/`
2. Implement execute method
3. Add to `application/use_cases/mod.rs`
4. Re-export in `lib.rs`

### Adding a new repository
1. Define trait in `domain/repositories/`
2. Implement in `infrastructure/database/repositories/`
3. Add DuckDB schema if needed

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run -p jira-db-cli -- <command>

# Enable trace logging
RUST_LOG=trace cargo run -p jira-db-cli -- <command>
```

## Common Issues

### DuckDB linking errors
Ensure DuckDB is installed and library paths are set:
```bash
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
```

### JIRA API errors
- Check endpoint URL includes `https://`
- Verify API token is correct (not password)
- Check API permissions in Atlassian admin
