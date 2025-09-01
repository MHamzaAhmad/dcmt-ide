use std::env;
use std::net::SocketAddr;

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
    
    tracing::info!("Starting WebTransport/WebSocket server on port {}", port);
    tracing::info!("Using database: {}", db_path);
    
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    // Create a simple HTTP server for now
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/ws", axum::routing::get(websocket_handler));
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("WebTransport server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> &'static str {
    "WebTransport/WebSocket server is running"
}

async fn websocket_handler() -> &'static str {
    "WebSocket endpoint (not implemented yet)"
}