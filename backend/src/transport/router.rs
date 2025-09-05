use crate::svc::FileService;
use crate::transport::middleware::{cors::create_cors_layer, logging::create_trace_layer};
use crate::transport::routes::{files_router, websocket_router};
use anyhow::Result;
use axum::Router;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn create_router() -> Result<Router> {
    // Initialize workspace directory
    let workspace_path = PathBuf::from("workspace");
    if !workspace_path.exists() {
        tokio::fs::create_dir_all(&workspace_path).await?;
    }
    
    // Create file service
    let file_service = Arc::new(FileService::new(workspace_path)?);
    
    // Create API routes by nesting sub-routers
    let api_routes = Router::new()
        .nest("/files", files_router())
        .with_state(file_service.clone());

    // Main application router
    let app = Router::new()
        .nest("/api", api_routes)
        .nest("/ws", websocket_router())
        .layer(create_cors_layer())
        .layer(create_trace_layer())
        .with_state(file_service);

    Ok(app)
}