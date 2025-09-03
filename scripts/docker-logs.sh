#!/bin/bash

# LaTeX IDE Docker Log Viewer
# Provides formatted real-time log viewing for the LaTeX IDE Docker container

CONTAINER_NAME="latex-ide"
DEFAULT_LINES=50

show_help() {
    cat << EOF
LaTeX IDE Docker Log Viewer

Usage: $0 [options] [container_name]

Options:
    -f, --follow     Follow log output (real-time)
    -n, --lines N    Show last N lines (default: $DEFAULT_LINES)
    -s, --service S  Filter logs for specific service
    -h, --help       Show this help message

Available services:
    web-frontend, webtransport-server, sse-handler, model-manager, latex-compiler

Examples:
    $0                           # Show last $DEFAULT_LINES lines
    $0 -f                        # Follow logs in real-time
    $0 -f -s webtransport-server # Follow logs for WebTransport server only
    $0 -n 100                    # Show last 100 lines
    $0 my-latex-container        # Use custom container name

EOF
}

# Parse command line arguments
FOLLOW=""
LINES=$DEFAULT_LINES
SERVICE_FILTER=""
CONTAINER="$CONTAINER_NAME"

while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--follow)
            FOLLOW="-f"
            shift
            ;;
        -n|--lines)
            LINES="$2"
            shift 2
            ;;
        -s|--service)
            SERVICE_FILTER="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
        *)
            CONTAINER="$1"
            shift
            ;;
    esac
done

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    exit 1
fi

# Check if container exists
if ! docker ps -a --format "table {{.Names}}" | grep -q "^${CONTAINER}$"; then
    echo "❌ Error: Container '$CONTAINER' not found"
    echo ""
    echo "Available containers:"
    docker ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    exit 1
fi

# Check if container is running
if ! docker ps --format "table {{.Names}}" | grep -q "^${CONTAINER}$"; then
    echo "⚠️  Warning: Container '$CONTAINER' is not running"
    echo ""
fi

# Build docker logs command
DOCKER_CMD="docker logs"

if [[ -n "$FOLLOW" ]]; then
    DOCKER_CMD="$DOCKER_CMD $FOLLOW"
fi

DOCKER_CMD="$DOCKER_CMD --tail $LINES"
DOCKER_CMD="$DOCKER_CMD $CONTAINER"

# Show header
echo "📋 LaTeX IDE Container Logs"
echo "============================"
echo "Container: $CONTAINER"
echo "Lines: $LINES"
if [[ -n "$SERVICE_FILTER" ]]; then
    echo "Service Filter: $SERVICE_FILTER"
fi
if [[ -n "$FOLLOW" ]]; then
    echo "Mode: Following (Press Ctrl+C to stop)"
fi
echo "============================"

# Execute command with optional service filtering
if [[ -n "$SERVICE_FILTER" ]]; then
    # Filter logs for specific service
    if [[ -n "$FOLLOW" ]]; then
        $DOCKER_CMD 2>&1 | grep -E "\[$SERVICE_FILTER\]|supervisord.*$SERVICE_FILTER" --line-buffered
    else
        $DOCKER_CMD 2>&1 | grep -E "\[$SERVICE_FILTER\]|supervisord.*$SERVICE_FILTER"
    fi
else
    # Show all logs
    $DOCKER_CMD 2>&1
fi