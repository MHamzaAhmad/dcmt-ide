use crate::{config::Config, svc::FileService};
use crate::transport::middleware::{cors::create_cors_layer, logging::create_trace_layer};
use crate::transport::routes::{files_router, websocket_router};
use anyhow::Result;
use axum::Router;
use std::sync::Arc;

pub async fn create_router(config: Config) -> Result<Router> {
    // Create file service with config
    let file_service = Arc::new(FileService::new(config.workspace_path)?);
    
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