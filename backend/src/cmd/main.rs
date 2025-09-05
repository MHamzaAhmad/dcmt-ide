use dcmt_backend::transport::router::create_router;
use anyhow::Result;
use axum::serve;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dcmt_backend=debug,tower_http=debug".into())
        )
        .init();

    // Ensure workspace directory exists
    let workspace_path = PathBuf::from("workspace");
    if !workspace_path.exists() {
        tokio::fs::create_dir_all(&workspace_path).await?;
        info!("Created workspace directory at: {:?}", workspace_path);
    } else {
        info!("Using existing workspace directory at: {:?}", workspace_path);
    }

    // Create router
    let app = create_router().await?;
    
    // Start server
    let listener = TcpListener::bind("127.0.0.1:3001").await?;
    info!("🚀 DCMT Backend server running on http://127.0.0.1:3001");
    info!("📁 Workspace path: {:?}", workspace_path.canonicalize().unwrap_or(workspace_path));
    info!("🔌 WebSocket endpoint: ws://127.0.0.1:3001/ws");
    
    serve(listener, app).await?;
    
    Ok(())
}