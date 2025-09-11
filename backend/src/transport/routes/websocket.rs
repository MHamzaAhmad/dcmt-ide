use crate::svc::{FileService, AgentService, LaTeXService};
use crate::transport::handlers::websocket;
use axum::{routing::get, Router};
use std::sync::Arc;

// Combined state for WebSocket handler to access file, agent and latex services
#[derive(Clone)]
pub struct WebSocketServices {
    pub file_service: Arc<FileService>,
    pub agent_service: Arc<AgentService>,
    pub latex_service: Arc<LaTeXService>,
}

pub fn websocket_router() -> Router<WebSocketServices> {
    Router::new()
        .route("/", get(websocket::websocket_handler))
}