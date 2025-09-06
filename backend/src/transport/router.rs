use crate::{config::Config, svc::{FileService, LaTeXService}};
use crate::transport::middleware::{cors::create_cors_layer, logging::create_trace_layer};
use crate::transport::routes::{files_router, latex_router, websocket_router};
use anyhow::Result;
use axum::Router;
use std::sync::Arc;

pub async fn create_router(config: Config) -> Result<Router> {
    // Create services with config
    let file_service = Arc::new(FileService::new(config.workspace_path.clone())?);
    let latex_service = Arc::new(LaTeXService::new(config.workspace_path));
    
    // Create separate routers for different services
    let files_routes = files_router().with_state(file_service.clone());
    let latex_routes = latex_router().with_state(latex_service);
    
    // Create API routes by nesting sub-routers
    let api_routes = Router::new()
        .nest("/files", files_routes)
        .nest("/latex", latex_routes);

    // Main application router
    let app = Router::new()
        .nest("/api", api_routes)
        .nest("/ws", websocket_router().with_state(file_service))
        .layer(create_cors_layer())
        .layer(create_trace_layer());

    Ok(app)
}