use axum::{
    routing::{get, post},
    middleware,
    Router,
};
use std::sync::Arc;

use crate::svc::AgentService;
use crate::transport::{
    handlers::agent::{chat_handler, list_tools_handler, session_info_handler},
    middleware::auth::auth_middleware,
};

/// Creates the agent router with all endpoints
pub fn agent_router() -> Router<Arc<AgentService>> {
    Router::new()
        // Chat endpoint - main agent interaction
        .route("/chat", post(chat_handler))
        // Tool information endpoints
        .route("/tools", get(list_tools_handler))
        // Session management endpoints
        .route("/session/:session_id", get(session_info_handler))
        // Apply authentication middleware to all agent routes
        .layer(middleware::from_fn(auth_middleware))
}