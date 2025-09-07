use crate::{config::Config, svc::{AgentService, FileService, LaTeXService}, repo::AgentRepo};
use crate::transport::middleware::{cors::create_cors_layer, logging::create_trace_layer};
use crate::transport::routes::{agent_router, files_router, latex_router, websocket_router};
use crate::transport::routes::websocket::WebSocketServices;
use anyhow::Result;
use axum::Router;
use std::sync::Arc;

pub async fn create_router(config: Config) -> Result<Router> {
    // Create services with config
    let file_service = Arc::new(FileService::new(config.workspace_path.clone())?);
    let latex_service = Arc::new(LaTeXService::new(config.workspace_path.clone()));
    
    // Create agent repository and service
    let agent_repo = Arc::new(AgentRepo::new(
        config.workspace_path.clone(),
        config.agent.litellm_base_url.clone(),
    ).await?);
    let agent_service = Arc::new(AgentService::new(agent_repo));
    
    // Create separate routers for different services
    let files_routes = files_router().with_state(file_service.clone());
    let latex_routes = latex_router().with_state(latex_service);
    let agent_routes = agent_router().with_state(agent_service.clone());
    
    // Create API routes by nesting sub-routers
    let api_routes = Router::new()
        .nest("/files", files_routes)
        .nest("/latex", latex_routes)
        .nest("/agent", agent_routes);

    // Create combined WebSocket services state
    let websocket_services = WebSocketServices {
        file_service: file_service.clone(),
        agent_service: agent_service,
    };
    
    // Main application router
    let app = Router::new()
        .nest("/api", api_routes)
        .nest("/ws", websocket_router().with_state(websocket_services))
        .layer(create_cors_layer())
        .layer(create_trace_layer());

    Ok(app)
}