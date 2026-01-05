# JiraDb Tauri Desktop App

Tauri-based desktop application for JiraDb.

## Prerequisites

- Rust (with Cargo)
- Node.js (with npm)
- Tauri CLI: `cargo install tauri-cli`

## Development

The Tauri app uses the shared frontend from `/frontend`.

### Development Server

```bash
# From the project root
cd crates/jira-db-tauri/src-tauri
cargo tauri dev
```

This will:
1. Start the Angular dev server with Tauri configuration
2. Launch the Tauri development window

### Debug Logging

To enable debug log output during development:

```bash
# Enable info level logs (default)
RUST_LOG=info cargo tauri dev

# Enable debug level logs (more verbose)
RUST_LOG=debug cargo tauri dev

# Enable trace level logs (most verbose)
RUST_LOG=trace cargo tauri dev
```

Logs are output to the terminal where `cargo tauri dev` is running.

### Building

```bash
# From the project root
cd crates/jira-db-tauri/src-tauri
cargo tauri build
```

This will:
1. Build the frontend with Tauri configuration
2. Build the Rust backend
3. Create platform-specific installers

## Architecture

- **Frontend**: Shared Angular app from `/frontend` with `tauri` build configuration
- **Backend**: Rust Tauri commands in `src-tauri/src/commands/`
- **Generated Types**: TypeSpec-generated types in `src-tauri/src/generated/`

## Configuration

The frontend build configuration is in `/frontend/angular.json`:
- `tauri`: Production build for Tauri
- `tauri-dev`: Development build for Tauri

Output is written to `/crates/jira-db-tauri/dist/`.
