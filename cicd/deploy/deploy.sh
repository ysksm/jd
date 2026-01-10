#!/bin/bash
#
# JIRA Mock Server Deployment Script
#
# Usage:
#   ./deploy.sh [options]
#
# Options:
#   -e, --env         Environment (staging|production) [default: staging]
#   -h, --host        Target host
#   -u, --user        SSH user
#   -p, --port        Application port [default: 8080]
#   -d, --data-dir    Data directory on host [default: /opt/jira-mock-server/data]
#   -i, --image       Docker image [default: ghcr.io/owner/repo/jira-mock-server:latest]
#   --build           Build Docker image locally before deploying
#   --dry-run         Print commands without executing
#   --help            Show this help message

set -euo pipefail

# Default values
ENV="staging"
HOST=""
USER=""
PORT="8080"
DATA_DIR="/opt/jira-mock-server/data"
IMAGE=""
BUILD=false
DRY_RUN=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    sed -n '3,16p' "$0" | sed 's/^#//'
    exit 0
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--env)
            ENV="$2"
            shift 2
            ;;
        -h|--host)
            HOST="$2"
            shift 2
            ;;
        -u|--user)
            USER="$2"
            shift 2
            ;;
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        -d|--data-dir)
            DATA_DIR="$2"
            shift 2
            ;;
        -i|--image)
            IMAGE="$2"
            shift 2
            ;;
        --build)
            BUILD=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            ;;
    esac
done

# Validate required parameters
if [[ -z "$HOST" ]]; then
    log_error "Host is required. Use -h or --host to specify."
    exit 1
fi

if [[ -z "$USER" ]]; then
    log_error "User is required. Use -u or --user to specify."
    exit 1
fi

# Set default image if not provided
if [[ -z "$IMAGE" ]]; then
    IMAGE="jira-mock-server:latest"
fi

# Set log level based on environment
if [[ "$ENV" == "production" ]]; then
    RUST_LOG="warn"
else
    RUST_LOG="info"
fi

log_info "Deployment Configuration:"
echo "  Environment: $ENV"
echo "  Host: $HOST"
echo "  User: $USER"
echo "  Port: $PORT"
echo "  Data Directory: $DATA_DIR"
echo "  Image: $IMAGE"
echo "  Log Level: $RUST_LOG"
echo ""

run_command() {
    if [[ "$DRY_RUN" == true ]]; then
        echo "[DRY-RUN] $1"
    else
        eval "$1"
    fi
}

# Build Docker image locally if requested
if [[ "$BUILD" == true ]]; then
    log_info "Building Docker image locally..."

    # Navigate to project root
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

    run_command "cd $PROJECT_ROOT && docker build -t $IMAGE -f cicd/deploy/Dockerfile ."

    if [[ "$DRY_RUN" != true ]]; then
        log_success "Docker image built successfully"
    fi
fi

# Deploy to remote server
log_info "Deploying to $HOST..."

DEPLOY_SCRIPT=$(cat << 'REMOTE_EOF'
#!/bin/bash
set -euo pipefail

IMAGE="__IMAGE__"
PORT="__PORT__"
DATA_DIR="__DATA_DIR__"
RUST_LOG="__RUST_LOG__"
CONTAINER_NAME="jira-mock-server"

echo "Creating data directory..."
mkdir -p "$DATA_DIR"

echo "Pulling latest image..."
docker pull "$IMAGE" || true

echo "Stopping existing container..."
docker stop "$CONTAINER_NAME" 2>/dev/null || true
docker rm "$CONTAINER_NAME" 2>/dev/null || true

echo "Starting new container..."
docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    -p "$PORT:8080" \
    -v "$DATA_DIR:/app/data" \
    -e "RUST_LOG=$RUST_LOG" \
    "$IMAGE"

echo "Waiting for service to start..."
sleep 5

echo "Checking service health..."
if curl -sf "http://localhost:$PORT/rest/api/3/project" > /dev/null; then
    echo "Service is healthy!"
else
    echo "Service health check failed!"
    docker logs "$CONTAINER_NAME" --tail 50
    exit 1
fi

echo "Cleaning up old images..."
docker image prune -f

echo "Deployment complete!"
REMOTE_EOF
)

# Replace placeholders
DEPLOY_SCRIPT="${DEPLOY_SCRIPT//__IMAGE__/$IMAGE}"
DEPLOY_SCRIPT="${DEPLOY_SCRIPT//__PORT__/$PORT}"
DEPLOY_SCRIPT="${DEPLOY_SCRIPT//__DATA_DIR__/$DATA_DIR}"
DEPLOY_SCRIPT="${DEPLOY_SCRIPT//__RUST_LOG__/$RUST_LOG}"

if [[ "$DRY_RUN" == true ]]; then
    log_info "Would execute the following script on $HOST:"
    echo "---"
    echo "$DEPLOY_SCRIPT"
    echo "---"
else
    echo "$DEPLOY_SCRIPT" | ssh "$USER@$HOST" 'bash -s'

    if [[ $? -eq 0 ]]; then
        log_success "Deployment to $ENV completed successfully!"
        log_info "Service URL: http://$HOST:$PORT"
    else
        log_error "Deployment failed!"
        exit 1
    fi
fi
