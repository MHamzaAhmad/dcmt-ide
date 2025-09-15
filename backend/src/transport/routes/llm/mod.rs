use axum::{
    routing::{get, post},
    Router,
};
use crate::repo::LiteLLMRepository;
use crate::transport::handlers::{llm_chat_handler, list_models_handler, health_check_handler};

/// Creates LLM router with authentication (handled manually in handlers)
pub fn llm_router(litellm_repo: LiteLLMRepository) -> Router {
    Router::new()
        .route("/models", get(list_models_handler))
        .route("/chat", post(llm_chat_handler))
        .route("/health", get(health_check_handler))
        .with_state(litellm_repo)
}