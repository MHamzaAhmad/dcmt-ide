use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::svc::LLMService;
use crate::transport::handlers::llm::list_models_handler;

/// Creates the LLM router with all endpoints
pub fn llm_router() -> Router<Arc<LLMService>> {
    Router::new()
        // Models endpoint - list available models
        .route("/models", get(list_models_handler))
}