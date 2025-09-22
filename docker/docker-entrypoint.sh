#!/bin/sh
set -e

# Ensure group-writable files/dirs by default
umask 0002

echo "🚀 Starting DCMT Editor Container (rootless)..."

# Create necessary directories
mkdir -p /app/logs

# Environment variable validation
echo "🔧 Validating configuration..."

if [ -z "$DCMT_HOST" ]; then
    export DCMT_HOST="0.0.0.0"
fi

if [ -z "$DCMT_PORT" ]; then
    export DCMT_PORT="3001"
fi

export DCMT_WORKSPACE_PATH="${DCMT_WORKSPACE_PATH:-/workspace}"

if [ -z "$LITELLM_BASE_URL" ]; then
    export LITELLM_BASE_URL="http://127.0.0.1:4000"
fi

echo "📊 Configuration:"
echo "  - Host: $DCMT_HOST"
echo "  - Port: $DCMT_PORT"
echo "  - Workspace: $DCMT_WORKSPACE_PATH"
echo "  - LiteLLM URL: $LITELLM_BASE_URL"
echo "  - User: $(id -u):$(id -g)"

# Create workspace if it doesn't exist (in case of volume mount)
mkdir -p "$DCMT_WORKSPACE_PATH"

# Check workspace availability
if [ -d "$DCMT_WORKSPACE_PATH" ]; then
    workspace_owner=$(stat -c '%U:%G' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    workspace_perms=$(stat -c '%a' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    echo "📁 Workspace status:"
    echo "  - Path: $DCMT_WORKSPACE_PATH"
    echo "  - Owner: $workspace_owner"
    echo "  - Permissions: $workspace_perms"
    if [ ! -w "$DCMT_WORKSPACE_PATH" ]; then
        echo "❌ Workspace is not writable by current user ($(id -u))"
        echo "➡️  Ensure your bind-mounted directory is owned by the same UID/GID as the container user (APP_UID/APP_GID) or adjust the mount." 
        exit 1
    fi
else
        echo "❌ Workspace directory does not exist: $DCMT_WORKSPACE_PATH"
    exit 1
fi

# Test backend binary
if [ ! -x "/app/backend/dcmt-backend" ]; then
    echo "❌ Backend binary not found or not executable!"
    exit 1
fi

echo "🧪 Testing backend binary..."
if ! timeout 2 /app/backend/dcmt-backend --help >/dev/null 2>&1; then
    echo "⚠️  Backend binary test skipped (no --help flag), continuing..."
fi

# Test nginx configuration
echo "🧪 Testing nginx configuration..."
if ! nginx -t -c /etc/nginx/nginx.conf; then
    echo "⚠️  Nginx configuration test reported issues (will continue and let supervisord capture logs)."
fi

echo "✅ Container initialization completed successfully!"
echo "🌐 Starting services..."
echo "  - Frontend: http://localhost:3000"
echo "  - API: http://localhost:3000/api/"
echo "  - LiteLLM: http://localhost:3000/llm/"
echo "  - WebSocket: ws://localhost:3000/ws"
echo ""
echo "📋 Note: SSL termination should be handled by host nginx"

# Run services as current user (rootless)
exec "$@"