#!/bin/sh
set -e

echo "🚀 Starting DCMT Editor Container..."

# Create necessary directories
mkdir -p /app/logs

# Note: /app/workspace is already created with proper ownership in Dockerfile
# We can't chown as non-root user, so permissions are handled during build

# Environment variable validation
echo "🔧 Validating configuration..."

if [ -z "$DCMT_HOST" ]; then
    export DCMT_HOST="0.0.0.0"
fi

if [ -z "$DCMT_PORT" ]; then
    export DCMT_PORT="3001"
fi

if [ -z "$DCMT_WORKSPACE_PATH" ]; then
    export DCMT_WORKSPACE_PATH="/app/workspace"
fi

if [ -z "$LITELLM_BASE_URL" ]; then
    export LITELLM_BASE_URL="http://127.0.0.1:4000"
fi

echo "📊 Configuration:"
echo "  - Host: $DCMT_HOST"
echo "  - Port: $DCMT_PORT"
echo "  - Workspace: $DCMT_WORKSPACE_PATH"
echo "  - LiteLLM URL: $LITELLM_BASE_URL"

# Create workspace if it doesn't exist (in case of volume mount)
mkdir -p "$DCMT_WORKSPACE_PATH"

# Check workspace permissions (for debugging)
if [ -d "$DCMT_WORKSPACE_PATH" ]; then
    workspace_owner=$(stat -c '%U:%G' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    workspace_perms=$(stat -c '%a' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    echo "📁 Workspace status:"
    echo "  - Path: $DCMT_WORKSPACE_PATH"
    echo "  - Owner: $workspace_owner"
    echo "  - Permissions: $workspace_perms"
    
    # Check if workspace is writable by current user
    if [ -w "$DCMT_WORKSPACE_PATH" ]; then
        echo "  - Writable: ✅ Yes"
    else
        echo "  - Writable: ❌ No"
        echo "⚠️  Warning: Workspace directory is not writable by appuser"
    fi
else
    echo "❌ Workspace directory does not exist: $DCMT_WORKSPACE_PATH"
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
    echo "❌ Nginx configuration test failed!"
    exit 1
fi

echo "✅ Container initialization completed successfully!"
echo "🌐 Starting services..."
echo "  - Frontend: http://localhost:3000"
echo "  - API: http://localhost:3000/api/"
echo "  - LiteLLM: http://localhost:3000/llm/"
echo "  - WebSocket: ws://localhost:3000/ws"
echo ""
echo "📋 Note: SSL termination should be handled by host nginx"

# Execute the command passed to the container
exec "$@"