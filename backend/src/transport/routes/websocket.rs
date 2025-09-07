use crate::svc::{FileService, AgentService};
use crate::transport::handlers::websocket;
use axum::{routing::get, Router};
use std::sync::Arc;

// Combined state for WebSocket handler to access both file and agent services
#[derive(Clone)]
pub struct WebSocketServices {
    pub file_service: Arc<FileService>,
    pub agent_service: Arc<AgentService>,
}

pub fn websocket_router() -> Router<WebSocketServices> {
    Router::new()
        .route("/", get(websocket::websocket_handler))
}