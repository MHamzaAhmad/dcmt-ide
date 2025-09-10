#!/bin/sh
set -e

echo "🚀 Starting DCMT Editor Container..."

# Create necessary directories
mkdir -p /app/logs /app/workspace

# Set permissions
chown -R appuser:appgroup /app/logs /app/workspace 2>/dev/null || true

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

# Create workspace if it doesn't exist and set permissions
mkdir -p "$DCMT_WORKSPACE_PATH"
chown -R appuser:appgroup "$DCMT_WORKSPACE_PATH"

# Fix permissions for mounted workspace volume
if [ -d "$DCMT_WORKSPACE_PATH" ]; then
    chown -R appuser:appgroup "$DCMT_WORKSPACE_PATH" 2>/dev/null || true
    chmod -R 755 "$DCMT_WORKSPACE_PATH" 2>/dev/null || true
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


# Final permissions check
chown -R appuser:appgroup /app/logs /app/workspace 2>/dev/null || true

echo "✅ Container initialization completed successfully!"
echo "🌐 Starting services..."
echo "  - Frontend: http://localhost:80"
echo "  - API: http://localhost:80/api/"
echo "  - LiteLLM: http://localhost:80/llm/"
echo "  - WebSocket: ws://localhost:80/ws"
echo ""
echo "📋 Note: SSL termination should be handled by host nginx"

# Execute the command passed to the container
exec "$@"