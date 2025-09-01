#!/bin/bash

# LaTeX IDE Development Script
# Usage: ./scripts/dev.sh [desktop|web|both|backend|full]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
log() {
    echo -e "${GREEN}[$(date +'%H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')] $1${NC}"
}

# Check dependencies
check_dependencies() {
    log "Checking dependencies..."
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        error "Rust is not installed. Please install Rust from https://rustup.rs/"
    fi
    
    # Check Dioxus CLI
    if ! command -v dx &> /dev/null; then
        warn "Dioxus CLI not found. Installing..."
        cargo install dioxus-cli
    fi
    
    # Check Docker (for web development)
    if ! command -v docker &> /dev/null; then
        warn "Docker not found. Web development requires Docker."
    fi
    
    # Check wasm-pack (for web builds)
    if ! command -v wasm-pack &> /dev/null; then
        warn "wasm-pack not found. Installing..."
        curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    fi
    
    log "Dependencies checked ✓"
}

# Create development certificates for WebTransport
create_dev_certs() {
    if [ ! -d "dev-certs" ] || [ ! -f "dev-certs/localhost.pem" ]; then
        log "Creating development certificates for WebTransport..."
        mkdir -p dev-certs
        
        # Create self-signed certificate for localhost
        openssl req -x509 -newkey rsa:2048 -keyout dev-certs/localhost-key.pem -out dev-certs/localhost.pem \
            -days 365 -nodes -subj "/CN=localhost" \
            -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
        
        log "Development certificates created ✓"
    fi
}

# Desktop development
dev_desktop() {
    log "🖥️  Starting Desktop Development..."
    
    cd apps/desktop
    
    info "Building and running desktop app with hot reload..."
    info "Press Ctrl+C to stop"
    
    # Use Dioxus CLI for hot reloading
    dx serve --platform desktop --hot-reload --open
}

# Web development
dev_web() {
    log "🌐 Starting Web Development..."
    
    # Ensure certificates exist
    create_dev_certs
    
    info "Building WASM module..."
    cd apps/web
    
    # Build WASM with development optimizations
    wasm-pack build --target web --out-dir pkg --dev
    
    cd ../..
    
    info "Starting Docker development environment..."
    docker compose -f docker-compose.dev.yml up --build web-dev
}

# Backend services
dev_backend() {
    log "⚙️  Starting Backend Services..."
    
    create_dev_certs
    
    info "Starting backend services with Docker..."
    docker compose -f docker-compose.dev.yml up --build backend-dev postgres redis latex-compiler
}

# Full stack development
dev_full() {
    log "🚀 Starting Full Development Stack..."
    
    create_dev_certs
    
    info "Starting all services..."
    docker compose -f docker-compose.dev.yml up --build
}

# Both desktop and web simultaneously
dev_both() {
    log "🔄 Starting Both Desktop and Web..."
    
    create_dev_certs
    
    # Start web services in background
    info "Starting web services in background..."
    docker compose -f docker-compose.dev.yml up -d --build web-dev backend-dev postgres redis
    
    # Wait a moment for services to start
    sleep 3
    
    # Start desktop in foreground
    info "Starting desktop app..."
    dev_desktop &
    
    # Keep script running
    wait
}

# Development tools and utilities
dev_tools() {
    log "🔧 Development Tools"
    echo "Available commands:"
    echo "  ./scripts/dev.sh desktop   - Desktop app with hot reload"
    echo "  ./scripts/dev.sh web       - Web app with Docker"
    echo "  ./scripts/dev.sh backend   - Backend services only"
    echo "  ./scripts/dev.sh both      - Desktop + Web simultaneously"
    echo "  ./scripts/dev.sh full      - Full development stack"
    echo "  ./scripts/dev.sh tools     - Show this help"
    echo "  ./scripts/dev.sh clean     - Clean build artifacts"
    echo "  ./scripts/dev.sh test      - Run tests"
    echo "  ./scripts/dev.sh fmt       - Format code"
    echo "  ./scripts/dev.sh clippy    - Run clippy lints"
}

# Clean build artifacts
dev_clean() {
    log "🧹 Cleaning build artifacts..."
    
    # Clean Rust builds
    cargo clean
    
    # Clean WASM builds
    rm -rf apps/web/pkg
    
    # Clean Docker volumes and images
    if command -v docker &> /dev/null; then
        docker compose -f docker-compose.dev.yml down -v --rmi local
        docker system prune -f
    fi
    
    log "Clean complete ✓"
}

# Run tests
dev_test() {
    log "🧪 Running tests..."
    
    # Run Rust tests
    cargo test --workspace
    
    # Run WASM tests
    cd apps/web
    wasm-pack test --headless --firefox
    cd ../..
    
    log "Tests complete ✓"
}

# Format code
dev_fmt() {
    log "📝 Formatting code..."
    cargo fmt --all
    log "Code formatted ✓"
}

# Run clippy
dev_clippy() {
    log "📎 Running Clippy..."
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    log "Clippy complete ✓"
}

# Watch for changes and restart services
dev_watch() {
    log "👀 Starting file watcher..."
    
    # Install watchexec if not available
    if ! command -v watchexec &> /dev/null; then
        cargo install watchexec-cli
    fi
    
    # Watch for changes in source files
    watchexec -r -e rs,toml,html,css,js -w crates -w apps -- docker compose -f docker-compose.dev.yml restart web-dev backend-dev
}

# Main script logic
main() {
    case "${1:-desktop}" in
        "desktop")
            check_dependencies
            dev_desktop
            ;;
        "web")
            check_dependencies
            dev_web
            ;;
        "backend")
            check_dependencies
            dev_backend
            ;;
        "both")
            check_dependencies
            dev_both
            ;;
        "full")
            check_dependencies
            dev_full
            ;;
        "tools"|"help"|"-h"|"--help")
            dev_tools
            ;;
        "clean")
            dev_clean
            ;;
        "test")
            check_dependencies
            dev_test
            ;;
        "fmt")
            dev_fmt
            ;;
        "clippy")
            dev_clippy
            ;;
        "watch")
            check_dependencies
            dev_watch
            ;;
        *)
            error "Unknown command: $1. Use 'tools' to see available commands."
            ;;
    esac
}

# Trap Ctrl+C to clean up background processes
cleanup() {
    info "Shutting down development environment..."
    docker compose -f docker-compose.dev.yml down
    exit 0
}

trap cleanup SIGINT SIGTERM

# Run main function
main "$@"