#!/bin/bash
set -e

# Docker entrypoint script for LaTeX IDE
# Handles service startup, logging, and graceful shutdown

echo "🚀 Starting LaTeX IDE Development Environment"
echo "=================================="

# Print environment info
echo "📦 Container: LaTeX IDE Multi-Service"
echo "🕒 Started at: $(date)"
echo "🔧 Services: web-frontend, webtransport-server, sse-handler, model-manager, latex-compiler"
echo "=================================="

# Ensure directories exist
mkdir -p /app/data /app/logs /app/workspace

# Set permissions
chown -R root:root /app/data /app/logs
chmod 755 /app/data /app/logs

# Print service startup information
echo "📋 Service Configuration:"
echo "  🌐 Web Frontend:        http://localhost:8080"
echo "  🔌 WebTransport Server: http://localhost:3001"  
echo "  📡 SSE Handler:         http://localhost:3002"
echo "  🤖 Model Manager:       http://localhost:3003"
echo "  📝 LaTeX Compiler:      http://localhost:3004"
echo "  💾 Database:           /app/data/latex_ide.db"
echo "  📂 Workspace:          /app/workspace"
echo "=================================="

# Function to handle shutdown
shutdown() {
    echo ""
    echo "🛑 Shutting down LaTeX IDE services..."
    supervisorctl -c /etc/supervisor/conf.d/latex-ide.conf shutdown
    echo "✅ Shutdown complete"
    exit 0
}

# Set up signal handlers for graceful shutdown
trap shutdown SIGTERM SIGINT

# Start services with supervisor
echo "🏁 Starting all services with Supervisor..."
echo "📝 Logs will be displayed below (use 'docker logs <container>' to view)"
echo "=================================="

# Start supervisord in foreground
exec /usr/bin/supervisord -n -c /etc/supervisor/conf.d/latex-ide.conf