#!/bin/bash
#
# Build script for JiraDb applications
#
# Usage:
#   ./scripts/build.sh          # Build all
#   ./scripts/build.sh cli      # Build CLI only
#   ./scripts/build.sh web      # Build Web server + Angular frontend
#   ./scripts/build.sh mcp      # Build MCP server only
#   ./scripts/build.sh tauri    # Build Tauri app
#   ./scripts/build.sh frontend # Build Angular frontend only

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Build Rust binaries
build_rust() {
    local target=$1
    info "Building Rust target: ${target:-all workspace members}"

    # Set library path for DuckDB on macOS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
        export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
    fi

    if [ -z "$target" ]; then
        # Build all workspace members (--workspace overrides default-members)
        cargo build --release --workspace
    else
        cargo build --release -p "$target"
    fi

    success "Rust build completed"
}

# Build Angular frontend
build_frontend() {
    local config=${1:-web}
    info "Building Angular frontend (configuration: $config)"

    cd "$PROJECT_ROOT/frontend"

    # Check if node_modules exists
    if [ ! -d "node_modules" ]; then
        info "Installing npm dependencies..."
        npm install
    fi

    npm run "build:$config"

    cd "$PROJECT_ROOT"
    success "Frontend build completed"
}

# Build CLI
build_cli() {
    build_rust "jira-db-cli"
}

# Build MCP server
build_mcp() {
    build_rust "jira-db-mcp"
}

# Build Web server (Rust + Angular)
build_web() {
    build_rust "jira-db-web"
    build_frontend "web"

    # Angular outputs directly to crates/jira-db-web/static/browser
    # No copy needed - angular.json has outputPath configured
    success "Web build completed. Static files at: crates/jira-db-web/static/browser"
}

# Build Tauri app
build_tauri() {
    build_frontend "tauri"

    info "Building Tauri application..."
    cd "$PROJECT_ROOT/crates/jira-db-tauri"

    # Check if node_modules exists
    if [ ! -d "node_modules" ]; then
        info "Installing npm dependencies..."
        npm install
    fi

    npm run tauri build

    cd "$PROJECT_ROOT"
    success "Tauri build completed"
}

# Build all applications
build_all() {
    info "Building all JiraDb applications..."

    # Build all Rust binaries
    build_rust ""

    # Build Angular frontend for web
    # Angular outputs directly to crates/jira-db-web/static/browser
    build_frontend "web"

    success "All builds completed!"
    echo ""
    info "Built binaries are available in: target/release/"
    info "  - jira-db (CLI)"
    info "  - jira-db-mcp (MCP Server)"
    info "  - jira-db-web (Web Server)"
    info ""
    info "Static files are available in: crates/jira-db-web/static/browser/"
}

# Main
case "${1:-all}" in
    all)
        build_all
        ;;
    cli)
        build_cli
        ;;
    web)
        build_web
        ;;
    mcp)
        build_mcp
        ;;
    tauri)
        build_tauri
        ;;
    frontend)
        build_frontend "${2:-web}"
        ;;
    *)
        echo "Usage: $0 {all|cli|web|mcp|tauri|frontend}"
        echo ""
        echo "Options:"
        echo "  all       Build all applications (default)"
        echo "  cli       Build CLI only"
        echo "  web       Build Web server + Angular frontend"
        echo "  mcp       Build MCP server only"
        echo "  tauri     Build Tauri desktop app"
        echo "  frontend  Build Angular frontend only (optional: config name)"
        exit 1
        ;;
esac
