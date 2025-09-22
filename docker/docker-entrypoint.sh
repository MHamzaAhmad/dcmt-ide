#!/bin/sh
set -e

# Ensure group-writable files/dirs by default (works with setgid bit on dirs)
umask 0002

echo "🚀 Starting DCMT Editor Container..."

# Create necessary directories
mkdir -p /app/logs

# If running as root, prepare env defaults and ensure workspace path exists
if [ "$(id -u)" = "0" ]; then
    : "${APP_USER:=appuser}"
    : "${APP_GROUP:=appgroup}"
    : "${DCMT_WORKSPACE_PATH:=/app/workspace}"
    mkdir -p "$DCMT_WORKSPACE_PATH"
fi

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
echo "  - User: $(id -u):$(id -g)"

# Create workspace if it doesn't exist (in case of volume mount)
mkdir -p "$DCMT_WORKSPACE_PATH"

# Check workspace permissions (for debugging)
if [ -d "$DCMT_WORKSPACE_PATH" ]; then
    workspace_owner=$(stat -c '%U:%G' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    workspace_perms=$(stat -c '%a' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    workspace_uid=$(stat -c '%u' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    workspace_gid=$(stat -c '%g' "$DCMT_WORKSPACE_PATH" 2>/dev/null || echo "unknown")
    echo "📁 Workspace status:"
    echo "  - Path: $DCMT_WORKSPACE_PATH"
    echo "  - Owner: $workspace_owner"
    echo "  - Permissions: $workspace_perms"
    echo "  - UID:GID: $workspace_uid:$workspace_gid"
    
    # If running as root, ensure libgit2 owner rules are satisfied:
    # - If workspace owned by root, take ownership as appuser so app can run as appuser
    # - If workspace owned by a non-root UID (bind mount), do not chown; we'll drop to that UID later
    if [ "$(id -u)" = "0" ]; then
        if [ "$workspace_uid" = "0" ]; then
            echo "🔑 Taking ownership of workspace for ${APP_USER}:${APP_GROUP}"
            chown -R ${APP_USER}:${APP_GROUP} "$DCMT_WORKSPACE_PATH" || true
            # After this, effective owner for matching should be appuser
        else
            echo "ℹ️  Workspace owned by non-root UID ($workspace_uid); will run as that UID"
        fi
        chmod 0775 "$DCMT_WORKSPACE_PATH" || true
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

# If we're root, drop privileges to appuser before launching services
if [ "$(id -u)" = "0" ]; then
    # Determine UID/GID to drop to: if workspace is owned by a non-root numeric UID,
    # run as that UID to satisfy libgit2 owner checks without chowning the host mount.
    if [ "$workspace_uid" != "unknown" ] && [ "$workspace_gid" != "unknown" ] && [ "$workspace_uid" != "0" ]; then
        echo "👤 Dropping privileges to workspace owner ${workspace_uid}:${workspace_gid} via gosu"
        exec gosu ${workspace_uid}:${workspace_gid} "$@"
    else
        echo "👤 Dropping privileges to ${APP_USER}:${APP_GROUP} via gosu"
        exec gosu ${APP_USER}:${APP_GROUP} "$@"
    fi
else
    # Execute the command passed to the container
    exec "$@"
fi