use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3002".to_string())
        .parse::<u16>()
        .unwrap_or(3002);
        
    let db_path = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///app/data/latex_ide.db".to_string());
    
    tracing::info!("Starting SSE handler server on port {}", port);
    tracing::info!("Using database: {}", db_path);
    
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    // Create a simple HTTP server for Server-Sent Events
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/events", axum::routing::get(sse_endpoint));
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("SSE handler listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> &'static str {
    "SSE handler server is running"
}

async fn sse_endpoint() -> &'static str {
    "Server-Sent Events endpoint (not implemented yet)"
}