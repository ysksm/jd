#!/bin/bash
#
# Simple deployment script for JiraDb Web application
#
# Usage:
#   ./scripts/deploy.sh /path/to/deploy     # Deploy to specified directory
#   ./scripts/deploy.sh                     # Deploy to ./deploy
#
# This script:
#   1. Builds the web application (Rust + Angular)
#   2. Copies all necessary files to the deployment directory
#   3. Creates a default config.toml if not exists
#   4. Generates a systemd service file (optional)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default deployment directory
DEPLOY_DIR="${1:-$PROJECT_ROOT/deploy}"

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

# Build web application
info "Building web application..."
"$SCRIPT_DIR/build.sh" web

# Create deployment directory structure
info "Creating deployment directory: $DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR/bin"
mkdir -p "$DEPLOY_DIR/static"
mkdir -p "$DEPLOY_DIR/data"
mkdir -p "$DEPLOY_DIR/logs"

# Copy binary
info "Copying jira-db-web binary..."
cp "$PROJECT_ROOT/target/release/jira-db-web" "$DEPLOY_DIR/bin/"
chmod +x "$DEPLOY_DIR/bin/jira-db-web"

# Copy static files
info "Copying static files..."
rm -rf "$DEPLOY_DIR/static/browser"
cp -r "$PROJECT_ROOT/static/browser" "$DEPLOY_DIR/static/"

# Create config.toml if not exists
if [ ! -f "$DEPLOY_DIR/config.toml" ]; then
    info "Creating default config.toml..."
    cat > "$DEPLOY_DIR/config.toml" << 'EOF'
# JiraDb Web Server Configuration

[server]
# Host address to bind to
# Use "0.0.0.0" to listen on all interfaces
host = "0.0.0.0"

# Port to bind to
port = 8080

[app]
# Path to JiraDb settings.json file
settings_path = "./data/settings.json"

# Path to static files directory (Angular build output)
static_dir = "./static/browser"
EOF
    success "Created config.toml"
else
    warn "config.toml already exists, skipping..."
fi

# Create run script
info "Creating run script..."
cat > "$DEPLOY_DIR/run.sh" << 'EOF'
#!/bin/bash
# Start JiraDb Web Server

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

exec ./bin/jira-db-web --config ./config.toml "$@"
EOF
chmod +x "$DEPLOY_DIR/run.sh"

# Create systemd service file (template)
info "Creating systemd service template..."
cat > "$DEPLOY_DIR/jira-db-web.service" << EOF
# JiraDb Web Server Systemd Service
#
# Installation:
#   1. Copy this file to /etc/systemd/system/jira-db-web.service
#   2. Edit the file to set correct paths and user
#   3. Run: sudo systemctl daemon-reload
#   4. Run: sudo systemctl enable jira-db-web
#   5. Run: sudo systemctl start jira-db-web

[Unit]
Description=JiraDb Web Server
After=network.target

[Service]
Type=simple
User=YOUR_USER
Group=YOUR_GROUP
WorkingDirectory=$DEPLOY_DIR
ExecStart=$DEPLOY_DIR/bin/jira-db-web --config $DEPLOY_DIR/config.toml
Restart=on-failure
RestartSec=5

# Logging
StandardOutput=append:$DEPLOY_DIR/logs/jira-db-web.log
StandardError=append:$DEPLOY_DIR/logs/jira-db-web.error.log

# Environment
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Create stop script
cat > "$DEPLOY_DIR/stop.sh" << 'EOF'
#!/bin/bash
# Stop JiraDb Web Server
pkill -f "jira-db-web" || echo "No running process found"
EOF
chmod +x "$DEPLOY_DIR/stop.sh"

# Summary
echo ""
success "Deployment completed!"
echo ""
info "Deployment directory: $DEPLOY_DIR"
echo ""
echo "Directory structure:"
echo "  $DEPLOY_DIR/"
echo "  ├── bin/"
echo "  │   └── jira-db-web"
echo "  ├── static/"
echo "  │   └── browser/"
echo "  ├── data/"
echo "  │   └── (settings.json will be created here)"
echo "  ├── logs/"
echo "  ├── config.toml"
echo "  ├── run.sh"
echo "  ├── stop.sh"
echo "  └── jira-db-web.service"
echo ""
echo "To start the server:"
echo "  cd $DEPLOY_DIR && ./run.sh"
echo ""
echo "To install as systemd service:"
echo "  1. Edit jira-db-web.service to set correct user/group"
echo "  2. sudo cp jira-db-web.service /etc/systemd/system/"
echo "  3. sudo systemctl daemon-reload"
echo "  4. sudo systemctl enable --now jira-db-web"
