use std::env;
use std::net::SocketAddr;
use axum::{
    extract::WebSocketUpgrade,
    response::Response,
    routing::get,
    Router,
};

use latex_ide_webtransport_server::websocket::{handle_websocket};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
        
    let db_path = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///app/data/latex_ide.db".to_string());
    
    let workspace_path = env::var("WORKSPACE_PATH")
        .unwrap_or_else(|_| "/workspace".to_string());
    
    tracing::info!("Starting WebTransport/WebSocket server on port {}", port);
    tracing::info!("Using database: {}", db_path);
    tracing::info!("Workspace path: {}", workspace_path);
    
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler));
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("WebTransport server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> &'static str {
    "WebTransport/WebSocket server is running"
}

async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_websocket)
}

