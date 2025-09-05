use dcmt_backend::{config::Config, transport::router::create_router};
use anyhow::Result;
use axum::serve;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dcmt_backend=debug,tower_http=debug".into())
        )
        .init();

    // Ensure workspace directory exists
    if !config.workspace_path.exists() {
        tokio::fs::create_dir_all(&config.workspace_path).await?;
        info!("Created workspace directory at: {:?}", config.workspace_path);
    } else {
        info!("Using existing workspace directory at: {:?}", config.workspace_path);
    }

    // Create router with config
    let app = create_router(config.clone()).await?;
    
    // Start server
    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&server_addr).await?;
    info!("🚀 DCMT Backend server running on http://{}", server_addr);
    info!("📁 Workspace path: {:?}", config.workspace_path.canonicalize().unwrap_or(config.workspace_path.clone()));
    info!("🔌 WebSocket endpoint: ws://{}/ws", server_addr);
    
    serve(listener, app).await?;
    
    Ok(())
}