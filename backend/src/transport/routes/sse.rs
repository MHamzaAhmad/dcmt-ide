use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::svc::AgentService;
use crate::transport::handlers::sse::{
    agent_events_handler,
    session_events_handler,
    sse_health_handler,
};

/// Creates the SSE router for Server-Sent Events streaming
pub fn sse_router() -> Router<Arc<AgentService>> {
    Router::new()
        // Global agent events stream (all sessions)
        .route("/agent/events", get(agent_events_handler))
        // Session-specific agent events stream
        .route("/agent/session/{session_id}/events", get(session_events_handler))
        // Health check for SSE endpoint
        .route("/health", get(sse_health_handler))
}