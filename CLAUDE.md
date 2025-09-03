# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LaTeX IDE is a modern, AI-powered collaborative LaTeX editor built with Rust and Dioxus. It features real-time collaboration via Yrs CRDT and WebTransport for ultra-low latency. The project provides both desktop (native) and web (WebAssembly) applications with identical functionality.

## Architecture

### Workspace Structure
```
latex-ide/
├── apps/
│   ├── desktop/          # Minimal Dioxus desktop app entry point
│   └── web/              # Minimal Dioxus web app entry point
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

**CRITICAL: App Organization Philosophy**

The `/apps/` directory contains **ONLY minimal entry points** - never add business logic directly to these apps:

- `apps/desktop/` and `apps/web/` should contain **ONLY**:
  - `main.rs` / `lib.rs` entry point
  - Basic app configuration and setup
  - Platform-specific initialization code
  - Dioxus app mounting logic

- **ALL FUNCTIONALITY** must be implemented in `/crates/` and imported into the apps
- **NO BUSINESS LOGIC** should exist directly in the app directories
- **MAXIMUM CODE SHARING** - identical functionality must use the same crate implementations

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

### App Development Rules

**NEVER add functionality directly to `/apps/` directories:**

❌ **What NOT to do:**
```rust
// apps/desktop/src/main.rs - WRONG!
fn main() {
    let editor_content = String::new();
    let git_status = check_git_status(); // Business logic in app - BAD!
    
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_title("LaTeX IDE")))
        .launch(app);
}

fn app() -> Element {
    rsx! {
        div { class: "main-container",  // UI in app - BAD!
            Editor { content: editor_content }
            GitPanel { status: git_status }
        }
    }
}
```

✅ **Correct approach:**
```rust
// apps/desktop/src/main.rs - CORRECT!
use latex_ide_ui::LaTeXApp;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_title("LaTeX IDE")))
        .launch(LaTeXApp); // All logic in shared crate
}
```

```rust
// crates/ui/src/lib.rs - CORRECT!
#[component]
pub fn LaTeXApp() -> Element {
    // All app logic, state management, and UI here
    // Shared between desktop and web platforms
}
```

### Dioxus Components
- Components use `#[component]` attribute and return `Element`
- State management with `use_signal()` for reactive updates
- Async operations with `wasm_bindgen_futures::spawn_local()` on web
- **ALL components must be in shared crates (`/crates/ui/`, etc.)**
- **Apps only import and mount components - never define them**

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
❌ "Add this component directly to apps/desktop/src/main.rs"
❌ "Put business logic in the app entry point"
❌ "Create separate implementations for web and desktop apps"

### Examples of Proper Approach

✅ Add todo in your todo list: "Fix git2 native dependency compatibility in Docker by implementing conditional compilation"
✅ Implement proper feature flags and conditional compilation
✅ Test that git functionality works in all environments where it should
✅ Provide meaningful error messages when git is unavailable
✅ "Create shared component in crates/ui and import it in both apps"
✅ "Move business logic to appropriate crate and expose through public API"
✅ "Implement functionality once in crates/ and share between platforms"

### Platform Compatibility Requirements

**BOTH APPLICATIONS MUST WORK AT ALL TIMES**

- Desktop app must always compile and run with full functionality
- Web app must always compile to WASM and run in browsers  
- Docker environment must build and run successfully
- Any changes must be tested on all platforms before being committed
- Platform-specific features must have appropriate fallbacks or error handling
- Shared functionality must work identically across platforms

## Code Architecture Summary

When working on this codebase:

1. **Keep apps minimal** - Only entry points and platform-specific initialization in `/apps/`
2. **Share maximum code** - All functionality implemented in `/crates/` and imported by both apps
3. **Use transport layer** - Backend communication through unified transport protocol
4. **Maintain cross-platform compatibility** - Both desktop and web must work identically
5. **Leverage development scripts** - Use existing build and development workflows
6. **Implement complete solutions** - No temporary fixes or simplified implementations
7. **Never duplicate logic** - If web and desktop need the same feature, create it once in a shared crate

**Remember: The apps should be so minimal that they're essentially just different compilation targets for the same shared codebase.**
- always compile wasm to check if every thing works or not

## UI Design System & Color Scheme

LaTeX IDE uses a sophisticated dark/light theme system with careful attention to accessibility and consistency across all components.

### Color Palette

The application uses **Zinc** as the primary color scale for a modern, professional appearance:

#### Light Theme
- **Backgrounds**: `bg-white`, `bg-zinc-50`, `bg-zinc-100`
- **Surfaces**: `bg-zinc-100` (secondary surfaces), `bg-zinc-200` (borders)
- **Text**: `text-zinc-900` (primary), `text-zinc-700` (secondary), `text-zinc-600` (tertiary), `text-zinc-500` (muted)
- **Interactive Elements**: `hover:bg-zinc-100`, `text-zinc-900` (active)

#### Dark Theme  
- **Backgrounds**: `bg-zinc-950`, `bg-zinc-900`, `bg-zinc-800`
- **Surfaces**: `bg-zinc-800` (secondary surfaces), `bg-zinc-700` (borders)
- **Text**: `text-zinc-100` (primary), `text-zinc-300` (secondary), `text-zinc-400` (tertiary), `text-zinc-500` (muted)
- **Interactive Elements**: `hover:bg-zinc-800`, `text-zinc-100` (active)

#### Accent Colors
- **Primary Buttons**: `bg-zinc-900` (light) / `bg-zinc-50` (dark) with inverted text
- **Success/Active**: `bg-blue-100` / `bg-blue-900`, `text-blue-600` / `text-blue-400`
- **Warning/Changes**: `bg-orange-500` (indicator dots)
- **Danger**: `bg-red-500` / `bg-red-900` for destructive actions

### Theme Implementation

The theming system uses:
- **CSS Classes**: TailwindCSS with `dark:` variants for automatic theme switching
- **Theme Context**: `use_theme()` hook provides access to current theme state
- **Local Storage**: Theme preference persisted in browser storage
- **Document Root**: `dark` class applied to `<html>` element for global theme switching

### Component Color Patterns

#### App Header (`crates/ui/src/web/app_header.rs`)
- **Background**: `bg-white dark:bg-zinc-900`
- **Border**: `border-zinc-200 dark:border-zinc-800`
- **Active Buttons**: `bg-zinc-900 text-zinc-50 dark:bg-zinc-50 dark:text-zinc-900`
- **Inactive Buttons**: `text-zinc-600 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100`
- **Hover States**: `hover:bg-zinc-100 dark:hover:bg-zinc-800`

#### File Manager (`crates/file-manager/src/web/components.rs`)
- **Background**: `bg-white dark:bg-zinc-900`
- **Tree Items**: `text-zinc-700 dark:text-zinc-300`
- **Hover States**: `hover:bg-zinc-100 dark:hover:bg-zinc-800`
- **Icons**: `text-zinc-500 dark:text-zinc-400`
- **Action Buttons**: `bg-white dark:bg-zinc-800` with `border-zinc-200 dark:border-zinc-700`

#### Editor Pane (`crates/editor/src/web/editor_pane.rs`)
- **Background**: `bg-white dark:bg-zinc-950`
- **Header**: `bg-zinc-50 dark:bg-zinc-900`
- **Toggle Buttons**: Active state uses `bg-zinc-900 dark:bg-zinc-100` with inverted text
- **Chat Toggle**: `bg-blue-100 dark:bg-blue-900` when active

#### Button Components (`crates/ui/src/button.rs`)
- **Primary**: `bg-zinc-900 dark:bg-zinc-50` with inverted text colors
- **Secondary**: `bg-zinc-100 dark:bg-zinc-800` with matching text
- **Ghost**: Transparent with `hover:bg-zinc-100 dark:hover:bg-zinc-800`
- **Danger**: `bg-red-500 dark:bg-red-900` for destructive actions

### Design Principles

1. **Consistent Contrast**: All text meets WCAG accessibility standards with proper contrast ratios
2. **Semantic Colors**: Colors have consistent meaning across components (zinc = neutral, blue = interactive/active, orange = warning, red = danger)
3. **Hover Feedback**: Interactive elements use consistent hover states with `transition-colors`
4. **Border Hierarchy**: `border-zinc-200/800` for primary borders, lighter variants for secondary
5. **Surface Elevation**: Background colors indicate visual hierarchy (darker = more elevated in dark mode)

### Usage Guidelines

When creating new components:
- Always provide both light and dark variants using TailwindCSS `dark:` prefix
- Use zinc scale for neutral colors, maintaining consistency with existing components
- Follow the established hover state patterns for interactive elements
- Test components in both themes to ensure proper contrast and visibility
- Use `text-zinc-500 dark:text-zinc-400` for muted/placeholder content
- Apply `transition-colors` to interactive elements for smooth theme transitions