#!/bin/sh
set -e

echo "🚀 Starting DCMT Editor Container..."

# Create necessary directories
mkdir -p /app/logs /app/ssl /app/workspace /var/www/certbot

# Set permissions
chown -R appuser:appgroup /app/logs /app/ssl /app/workspace 2>/dev/null || true

# SSL Certificate Setup
echo "🔒 Setting up SSL certificates..."

if [ -f "/app/ssl/cert.pem" ] && [ -f "/app/ssl/key.pem" ]; then
    echo "✓ SSL certificates found, using existing certificates"
elif [ ! -z "$SSL_CERT_PATH" ] && [ ! -z "$SSL_KEY_PATH" ]; then
    echo "📋 Copying SSL certificates from environment paths..."
    cp "$SSL_CERT_PATH" /app/ssl/cert.pem
    cp "$SSL_KEY_PATH" /app/ssl/key.pem
    chown appuser:appgroup /app/ssl/cert.pem /app/ssl/key.pem
    echo "✓ SSL certificates copied successfully"
elif [ ! -z "$DOMAIN" ]; then
    echo "🌐 Domain specified: $DOMAIN"
    echo "🔄 For Let's Encrypt, mount certificates to /app/ssl/ or use certbot"
    echo "🔧 Generating temporary self-signed certificate..."
    /app/generate-ssl.sh
else
    echo "🔧 Generating self-signed SSL certificate..."
    /app/generate-ssl.sh
fi

# Verify SSL certificates exist
if [ ! -f "/app/ssl/cert.pem" ] || [ ! -f "/app/ssl/key.pem" ]; then
    echo "❌ SSL certificates not found! Generating emergency self-signed certificate..."
    /app/generate-ssl.sh
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

# Create workspace if it doesn't exist and set permissions
mkdir -p "$DCMT_WORKSPACE_PATH"
chown -R appuser:appgroup "$DCMT_WORKSPACE_PATH"

# Fix permissions for mounted workspace volume
if [ -d "$DCMT_WORKSPACE_PATH" ]; then
    sudo chown -R appuser:appgroup "$DCMT_WORKSPACE_PATH" 2>/dev/null || true
    sudo chmod -R 755 "$DCMT_WORKSPACE_PATH" 2>/dev/null || true
fi

# Test backend binary
if [ ! -x "/app/backend/dcmt-backend" ]; then
    echo "❌ Backend binary not found or not executable!"
    exit 1
fi

echo "🧪 Testing backend binary..."
if ! /app/backend/dcmt-backend --help >/dev/null 2>&1; then
    echo "⚠️  Backend binary test failed, but continuing..."
fi

# Test nginx configuration
echo "🧪 Testing nginx configuration..."
if ! nginx -t -c /etc/nginx/nginx.conf; then
    echo "❌ Nginx configuration test failed!"
    exit 1
fi

# Final permissions check
chown -R appuser:appgroup /app/logs /app/ssl /app/workspace 2>/dev/null || true

echo "✅ Container initialization completed successfully!"
echo "🌐 Starting services..."
echo "  - Frontend: https://localhost:443"
echo "  - API: https://localhost:443/api/"
echo "  - LiteLLM: https://localhost:443/llm/"
echo "  - WebSocket: wss://localhost:443/ws"

# Execute the command passed to the container
exec "$@"