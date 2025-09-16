use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::repo::llm::LLMModelsResponse;
use crate::svc::LLMService;

/// Handler for listing available models
pub async fn list_models_handler(
    State(service): State<Arc<LLMService>>,
) -> Result<Json<LLMModelsResponse>, (StatusCode, String)> {
    info!("Handling list models request");

    match service.list_models().await {
        Ok(models) => {
            info!("Successfully retrieved {} models", models.data.len());
            Ok(Json(models))
        }
        Err(e) => {
            error!("Failed to list models: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve models: {}", e),
            ))
        }
    }
}