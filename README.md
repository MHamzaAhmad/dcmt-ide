# LaTeX IDE

A modern, AI-powered collaborative LaTeX editor built with Rust and Dioxus, featuring real-time collaboration via Yrs CRDT and WebTransport for ultra-low latency.

## 🚀 Features

### Core Editor
- **Native Performance**: Desktop app with full Rust performance
- **WebAssembly Web App**: High-performance browser-based editor
- **Syntax Highlighting**: LaTeX-aware syntax highlighting with tree-sitter
- **Real-time Collaboration**: Conflict-free collaborative editing with Yrs CRDT
- **PDF Preview**: Live compilation and preview with TinyTeX

### AI Integration
- **Multiple Model Support**: Choose from local (Kalosm) and remote (OpenAI, Anthropic, Gemini) models
- **LaTeX-Aware**: Context-aware LaTeX code generation and assistance
- **Streaming Responses**: Real-time AI responses via Server-Sent Events
- **Cost Tracking**: Monitor token usage and costs across models

### Advanced Connectivity
- **WebTransport**: Ultra-low latency with multiplexed streams (23% better than WebSockets)
- **WebSocket Fallback**: Automatic fallback for older browsers
- **Cross-Platform**: Identical experience on desktop and web

## 🏗️ Architecture

```
latex-ide/
├── apps/
│   ├── desktop/          # Dioxus desktop app (native Yrs)
│   └── web/             # Dioxus web app (ywasm bindings)
├── crates/
│   ├── ui/              # Shared UI components (TailwindCSS)
│   ├── editor/          # Text editor with syntax highlighting
│   ├── yrs-collab/      # Native Rust Yrs CRDT collaboration
│   ├── webtransport-server/  # WebTransport backend server
│   ├── sse-handler/     # Server-Sent Events for AI streaming
│   ├── model-manager/   # Multi-model AI management
│   ├── latex-compiler/  # TinyTeX integration
│   └── pdf-viewer/      # PDF rendering component
└── docker/              # Docker configurations
```

## 🛠️ Development Setup

### Prerequisites

- **Rust** 1.89+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Docker** (for web development)
- **Node.js** 18+ (for web tooling)

### Quick Start

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd latex-ide
   ```

2. **Install dependencies**
   ```bash
   # Install Dioxus CLI
   cargo install dioxus-cli
   
   # Install wasm-pack (for web builds)
   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
   
   # Install Node.js dependencies
   npm install
   ```

3. **Start development**
   ```bash
   # Desktop app with hot reloading
   ./scripts/dev.sh desktop
   
   # Web app with Docker
   ./scripts/dev.sh web
   
   # Both simultaneously
   ./scripts/dev.sh both
   
   # Full stack (including backend services)
   ./scripts/dev.sh full
   ```

### Development Commands

| Command | Description |
|---------|-------------|
| `./scripts/dev.sh desktop` | Start desktop app with hot reload |
| `./scripts/dev.sh web` | Start web app with Docker |
| `./scripts/dev.sh backend` | Start backend services only |
| `./scripts/dev.sh both` | Desktop + Web simultaneously |
| `./scripts/dev.sh full` | Full development stack |
| `./scripts/dev.sh clean` | Clean all build artifacts |
| `./scripts/dev.sh test` | Run all tests |
| `./scripts/dev.sh fmt` | Format code |
| `./scripts/dev.sh clippy` | Run Clippy lints |

### NPM Scripts

```bash
# Development
npm run dev:desktop     # Desktop app
npm run dev:web        # Web app  
npm run dev:both       # Both apps

# Building
npm run build:desktop  # Build desktop release
npm run build:web      # Build web release

# Testing
npm run test           # Run all tests
npm run test:web       # Web-specific tests
npm run test:desktop   # Desktop-specific tests

# Docker
npm run docker:up      # Start all services
npm run docker:down    # Stop all services
npm run docker:logs    # View logs
```

## 🌐 Web Development

The web version uses WebAssembly for high performance:

1. **Start web development**
   ```bash
   ./scripts/dev.sh web
   ```

2. **Access the application**
   - Web app: http://localhost:8080
   - Backend API: http://localhost:3000
   - WebTransport: https://localhost:4433

3. **Docker services include**
   - Web app with live reloading
   - WebTransport server
   - PostgreSQL database
   - Redis cache
   - LaTeX compilation service

## 🖥️ Desktop Development

The desktop app provides native performance:

1. **Start desktop development**
   ```bash
   ./scripts/dev.sh desktop
   ```

2. **Features**
   - Native Yrs CRDT (no WebAssembly overhead)
   - Hot reloading with `dx serve`
   - File system integration
   - Native menu and keyboard shortcuts

## 🤖 AI Configuration

### Local Models (Kalosm)
- **Llama 3.1/3.2**: Code generation and chat
- **Mistral 7B**: Fast responses
- **CodeLlama**: Specialized for LaTeX
- **Phi-3**: Lightweight model

### Remote Models
Configure API keys in environment:
```bash
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"  
export GEMINI_API_KEY="your-key"
```

### Model Selection
- Real-time switching between models
- Performance metrics display
- Cost tracking per model
- Context preservation across switches

## 🏗️ Building for Production

### Desktop Builds
```bash
cd apps/desktop
dx build --release --platform desktop
```

Outputs platform-specific binaries in `apps/desktop/dist/`

### Web Builds
```bash
cd apps/web
wasm-pack build --target web --out-dir pkg --release
```

Creates optimized WebAssembly bundle in `apps/web/pkg/`

### Docker Deployment
```bash
# Build production images
docker-compose -f docker-compose.prod.yml build

# Deploy
docker-compose -f docker-compose.prod.yml up -d
```

## 🧪 Testing

### Unit Tests
```bash
cargo test --workspace
```

### Web Tests
```bash
cd apps/web
wasm-pack test --headless --firefox
```

### Integration Tests
```bash
cargo test --workspace --test integration
```

### Performance Tests
```bash
cargo bench --workspace
```

## 📊 Performance

### WebTransport Benefits
- **23% lower latency** compared to WebSockets
- **Multiplexed streams** prevent head-of-line blocking
- **Better mobile performance** with network handovers
- **Automatic fallback** to WebSockets when not supported

### Yrs CRDT Advantages
- **Native Rust performance** on desktop
- **WebAssembly optimized** for web
- **Conflict-free** collaborative editing
- **Offline-first** with sync when connected

## 🔧 Configuration

### Environment Variables
```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/latex_ide

# Redis
REDIS_URL=redis://localhost:6379

# AI Models
OPENAI_API_KEY=your-openai-key
ANTHROPIC_API_KEY=your-anthropic-key
GEMINI_API_KEY=your-gemini-key

# TinyTeX
TINYTEX_ROOT=/usr/local/tinytex

# Certificates (for WebTransport)
CERT_PATH=/path/to/cert.pem
KEY_PATH=/path/to/key.pem
```

### LaTeX Configuration
- **Engines**: pdflatex, xelatex, lualatex
- **Packages**: Automatic installation of missing packages
- **Bibliography**: bibtex, biber support
- **Output**: PDF, DVI, PS formats

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes
- Add tests for new functionality
- Update documentation

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Dioxus**: Modern UI framework for Rust
- **Yrs**: Rust port of Y.js CRDT
- **TinyTeX**: Lightweight LaTeX distribution
- **Kalosm**: Local AI model runner
- **WebTransport**: Modern web protocol for low latency

## 📚 Documentation

- [Architecture Guide](docs/ARCHITECTURE.md)
- [API Documentation](docs/API.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Contributing Guidelines](docs/CONTRIBUTING.md)

## 🐛 Bug Reports

Please report bugs using GitHub Issues with:
- Steps to reproduce
- Expected vs actual behavior
- System information (OS, browser, versions)
- Console logs if applicable

## 💬 Community

- **GitHub Discussions**: Questions and feature requests
- **Issues**: Bug reports and technical problems
- **Pull Requests**: Code contributions

---

**Built with ❤️ using Rust, Dioxus, and modern web technologies**