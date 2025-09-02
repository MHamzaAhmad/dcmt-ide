# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LaTeX IDE is a modern, AI-powered collaborative LaTeX editor built with Rust and Dioxus. It features real-time collaboration via Yrs CRDT and WebTransport for ultra-low latency. The project provides both desktop (native) and web (WebAssembly) applications with identical functionality.

## Architecture

### Workspace Structure
```
latex-ide/
├── apps/
│   ├── desktop/          # Dioxus desktop app (native performance)
│   └── web/              # Dioxus web app (WASM compilation)
├── crates/
│   ├── ui/               # Shared UI components with TailwindCSS
│   ├── editor/           # Text editor with LaTeX syntax highlighting
│   ├── file-manager/     # File operations and Git integration
│   ├── git-manager/      # Git operations backend
│   ├── version-control-ui/ # Git UI components
│   ├── pdf-viewer/       # PDF preview and rendering
│   ├── chat/             # AI chat integration
│   ├── yrs-collab/       # CRDT-based real-time collaboration
│   ├── webtransport-server/ # Backend WebTransport/WebSocket server
│   ├── sse-handler/      # Server-Sent Events for AI streaming
│   ├── model-manager/    # Multi-model AI management
│   └── latex-compiler/   # LaTeX compilation service
```

### Key Technologies
- **Frontend**: Dioxus (Rust UI framework), TailwindCSS
- **Backend**: Axum web server, WebTransport/WebSocket for real-time communication
- **Database**: SQLite for local storage, Yrs CRDT for collaboration state
- **AI Integration**: Multiple model support (local Kalosm models + remote APIs)
- **PDF**: Native PDF rendering with pdfium-render
- **Git**: Full version control via git2 crate with session-based workflow

### Transport Layer Architecture

The project uses a sophisticated transport layer with platform-specific optimizations:

**Web Platform (WASM):**
- **Browser Capability Detection**: Infrastructure for detecting WebTransport support (`use_browser_capabilities`)
- **WebSocket Current**: Currently uses WebSocket with capability detection framework in place
- **WebTransport Ready**: Architecture prepared for WebTransport implementation with automatic fallback
- **Unified API**: Same transport interface prepared for both protocols

**Desktop Platform (Native):**
- **WebTransport Primary**: Uses native WebTransport via `wtransport` crate for maximum performance
- **Direct Connection**: No browser limitations, full WebTransport feature support
- **Native TLS**: Uses system certificate store and native TLS implementation

**Transport Layer Implementation:**
```rust
// Platform-specific transport implementations
#[cfg(target_arch = "wasm32")]
pub struct Transport {
    websocket: Option<WebSocket>,        // WebSocket fallback for web
    // Detects and tries WebTransport first, falls back to WebSocket
}

#[cfg(not(target_arch = "wasm32"))]  
pub struct Transport {
    connection: Option<Connection>,      // Native WebTransport for desktop
    // Uses wtransport crate directly for optimal performance
}
```

**Key Features:**
- **Protocol Selection Framework**: Web has infrastructure for capability-based protocol selection
- **Unified Message Format**: Same `TransportMessage` enum across all platforms
- **Git Operations**: All Git functionality runs on backend server, accessible via transport layer
- **Real-time Collaboration**: CRDT synchronization via transport layer
- **File Operations**: Backend file system access through transport protocol

**Current Status:**
- Desktop: Full WebTransport implementation via `wtransport` crate
- Web: WebSocket implementation with WebTransport capability detection framework
- Both platforms use identical `TransportMessage` protocol for backend communication

## Development Commands

### Primary Development Script
Use `./scripts/dev.sh` as the main entry point:

```bash
# Desktop development with hot reload
./scripts/dev.sh desktop

# Web development with Docker (includes all backend services)
./scripts/dev.sh web

# Run desktop and web simultaneously
./scripts/dev.sh both

# Full development stack (all services)
./scripts/dev.sh full

# Backend services only
./scripts/dev.sh backend
```

### NPM Scripts (Alternative)
```bash
# Development
npm run dev:desktop     # Desktop app
npm run dev:web         # Web app with Docker
npm run dev:both        # Both simultaneously

# Building
npm run build:web       # WASM build for web
npm run build:desktop   # Native desktop build

# Testing
npm run test           # All tests
npm run test:web       # WASM tests only

# Code quality
npm run fmt            # Format code
npm run clippy         # Run lints

# Docker management
npm run docker:up      # Start services
npm run docker:down    # Stop services
```

### Direct Cargo Commands
```bash
# Build desktop app
cd apps/desktop && dx serve --platform desktop

# Build web WASM module
cd apps/web && wasm-pack build --target web --out-dir pkg --dev

# Build backend services
cargo build --bin webtransport-server --bin sse-handler --bin model-manager

# Run tests
cargo test --workspace

# Format and lint
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Build System Notes

### WASM Compilation
- Web app compiles to WebAssembly using `wasm-pack`
- Uses `wasm32-unknown-unknown` target
- Development builds include debug symbols
- Production builds are optimized for size

### Docker Development
- Unified development environment with all services in one container
- Volume mounts for hot reloading of Rust source code
- Workspace directory mounted for file operations
- SQLite database persisted in Docker volume
- All ports exposed: 8080 (web), 3001 (WebTransport), 3002 (SSE), 3003 (Model Manager)

### Git Integration
- **Architecture**: Git operations run on backend server, accessed via transport layer
- **Session-based**: Each editing session creates a temporary branch (e.g., `session-123`)
- **Auto-versioning**: Automatic commits with version tags (v1.0.1, v1.0.2, etc.)
- **Docker Compatibility**: Git functionality disabled in Docker builds due to native dependencies

## Code Patterns

### Crate Organization Structure

**Each crate follows a strict organization pattern:**

```
crates/example-crate/
├── src/
│   ├── lib.rs          # Common functionality shared by both platforms
│   ├── web/            # Web-specific implementations and modules
│   │   ├── mod.rs
│   │   └── *.rs
│   └── desktop/        # Desktop-specific implementations and modules
│       ├── mod.rs
│       └── *.rs
```

**Key principles:**
- **Common code in `src/lib.rs`** - Platform-agnostic functionality 
- **Web-specific in `src/web/`** - WASM-compatible implementations, WebTransport usage
- **Desktop-specific in `src/desktop/`** - Native implementations, direct system access
- **Both platforms must work at all times** - Never break one platform for the other
- **Shared interfaces** - Common traits and types in lib.rs, platform-specific implementations

**Example structure:**
```rust
// lib.rs - Common traits and shared functionality
pub trait FileManager {
    async fn read_file(&self, path: &str) -> Result<String>;
}

// web/mod.rs - Web implementation using transport layer
pub struct WebFileManager;
impl FileManager for WebFileManager { /* WebTransport implementation */ }

// desktop/mod.rs - Desktop implementation using native APIs  
pub struct DesktopFileManager;
impl FileManager for DesktopFileManager { /* Direct file system access */ }
```

### Dioxus Components
- Components use `#[component]` attribute and return `Element`
- State management with `use_signal()` for reactive updates
- Async operations with `wasm_bindgen_futures::spawn_local()` on web
- Cross-platform components in shared crates (`/crates/ui/`)

### Transport Layer Usage
```rust
// Send git operation via transport layer
use latex_ide_file_manager::web::transport;
let response = transport::send_git_operation(transport::GitOp::GetStatus).await?;
```

### Error Handling
- Use `anyhow::Result` for general errors
- Use `thiserror` for custom error types
- Comprehensive logging with `tracing` crate

### WebTransport Messages
```rust
#[derive(Serialize, Deserialize)]
pub enum TransportMessage {
    GitOperation(GitOp),
    GitResponse(GitResponseData),
    FileOperation(FileOp),
    CollaborationSync(YrsUpdate),
}
```

## Testing Strategy

### Unit Tests
```bash
cargo test --workspace
```

### WASM Tests
```bash
cd apps/web && wasm-pack test --headless --firefox
```

### Integration Tests
Focus on transport layer communication and Git operations:
```bash
cargo test --test integration
```

## Common Development Issues

### Docker Build Failures
If webtransport-server fails to compile in Docker:
1. Check if git-manager dependency is causing native library conflicts
2. Use `--no-default-features` flag to disable git integration in Docker
3. Ensure conditional compilation flags are properly set

### WASM Compilation Errors
- Ensure all web-specific code uses `#[cfg(target_arch = "wasm32")]`
- Check that git2 dependencies are not included in WASM builds
- Use transport layer for all backend operations from web frontend

### Hot Reload Issues
- Desktop: Use `dx serve` with `--hot-reload` flag
- Web: Ensure Docker volumes are properly mounted for source code
- Both: Check that file watchers are detecting changes in mounted directories

## Environment Variables

```bash
# Database
DATABASE_URL=sqlite:///app/data/latex_ide.db

# AI Models (optional)
OPENAI_API_KEY=your-key
ANTHROPIC_API_KEY=your-key  
GEMINI_API_KEY=your-key

# Development
RUST_LOG=debug
RUST_BACKTRACE=1
NODE_ENV=development
```

## Important Implementation Notes

### Cross-Platform Git
- Git operations work identically on desktop and web via transport layer
- Backend server provides git functionality to both platforms
- Session-based workflow prevents conflicts in multi-user environments
- All Git UI components use same transport protocol

### Real-time Collaboration
- Yrs CRDT handles conflict-free collaborative editing
- Native Yrs performance on desktop, ywasm bindings for web
- Document state synchronized via WebTransport for minimal latency

### AI Integration
- Multiple model backends supported simultaneously
- Streaming responses via Server-Sent Events
- Cost tracking and performance metrics per model
- Context preservation across model switches

## Development Philosophy

**NO SIMPLIFIED IMPLEMENTATIONS OR TEMPORARY DISABLING**

This codebase follows a strict philosophy of proper implementation:

- **NO "simplified" implementations** - Always implement full functionality
- **NO "temporary" disabling** - Never disable features as a quick fix
- **NO "for now" solutions** - Avoid temporary workarounds
- **NO incomplete TODOs** - If there's a problem, fix it completely

### Problem Resolution Approach

When encountering issues:

1. **Add a specific todo** describing the exact problem and solution needed
2. **Implement the complete fix** handling all test cases and edge cases  
3. **Test thoroughly** across all platforms (desktop, web, Docker)
4. **Ensure cross-platform compatibility** - the same functionality must work everywhere
5. **Handle all error scenarios** with proper error messages and fallbacks

### Examples of What NOT to Do

❌ "Temporarily comment out git integration for Docker"
❌ "Simplified implementation without error handling" 
❌ "Disable this feature for now"
❌ "Quick fix - will improve later"

### Examples of Proper Approach

✅ Add todo in your todo list: "Fix git2 native dependency compatibility in Docker by implementing conditional compilation"
✅ Implement proper feature flags and conditional compilation
✅ Test that git functionality works in all environments where it should
✅ Provide meaningful error messages when git is unavailable

### Platform Compatibility Requirements

**BOTH APPLICATIONS MUST WORK AT ALL TIMES**

- Desktop app must always compile and run with full functionality
- Web app must always compile to WASM and run in browsers  
- Docker environment must build and run successfully
- Any changes must be tested on all platforms before being committed
- Platform-specific features must have appropriate fallbacks or error handling
- Shared functionality must work identically across platforms

When working on this codebase, prefer using the transport layer for backend communication, maintain cross-platform compatibility, leverage the existing development scripts for efficient workflows, and always implement complete, robust solutions rather than temporary fixes.