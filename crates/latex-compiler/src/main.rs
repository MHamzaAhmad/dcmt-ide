use clap::Parser;
use dcmt_ide_latex_compiler::*;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::{info, error};

#[derive(Parser, Debug)]
#[command(name = "latex-compiler")]
#[command(about = "LaTeX Compiler Service for LaTeX IDE")]
struct Args {
    #[arg(long, default_value = "3004")]
    port: u16,
    
    #[arg(long, default_value = "/tmp")]
    temp_dir: PathBuf,
    
    #[arg(long)]
    latex_bin_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("latex_compiler=info,dcmt_ide_latex_compiler=debug")
        .init();

    let args = Args::parse();

    // Add LaTeX binaries to PATH if specified
    if let Some(latex_bin_path) = &args.latex_bin_path {
        if latex_bin_path.exists() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let new_path = format!("{}:{}", latex_bin_path.display(), current_path);
            std::env::set_var("PATH", new_path);
        } else {
            info!("Custom LaTeX binaries path does not exist: {}", latex_bin_path.display());
        }
    } else {
        info!("Using system LaTeX binaries from PATH (TexLive recommended)");
    }

    // Create compiler service
    let compiler = LaTeXCompiler::new(args.temp_dir);
    
    // Start WebSocket server for LaTeX compilation requests
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("LaTeX Compiler Service listening on {}", addr);
    info!("Ready to accept WebTransport/WebSocket connections");
    info!("Integration: Connect via WebTransport server on port 3001");

    // Keep the service running
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Received connection from {}", addr);
                let compiler_clone = compiler.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, compiler_clone).await {
                        error!("Connection error: {}", e);
                    }
                });
            },
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn verify_latex_installation() -> anyhow::Result<Vec<Engine>> {
    let mut available_engines = Vec::new();
    
    for engine in [Engine::PdfLatex, Engine::XeLatex, Engine::LuaLatex] {
        match tokio::process::Command::new(engine.command())
            .arg("--version")
            .output()
            .await 
        {
            Ok(output) if output.status.success() => {
                available_engines.push(engine);
                info!("{} is available", engine.command());
            },
            Ok(_) => {
                info!("{} returned non-zero exit code", engine.command());
            },
            Err(_) => {
                info!("{} not found in PATH", engine.command());
            }
        }
    }
    
    if available_engines.is_empty() {
        anyhow::bail!("No LaTeX engines found. Please install LaTeX (TeX Live or MiKTeX)");
    }
    
    Ok(available_engines)
}

async fn handle_connection(
    _stream: tokio::net::TcpStream, 
    _compiler: LaTeXCompiler
) -> anyhow::Result<()> {
    // This is a placeholder for WebSocket/WebTransport integration
    // The actual connection handling should integrate with the existing
    // WebTransport server infrastructure in crates/webtransport-server
    info!("Connection handler placeholder - integrate with WebTransport server");
    Ok(())
}