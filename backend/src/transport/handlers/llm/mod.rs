use crate::repo::litellm::*;
use axum::{
    extract::State,
    http::{header::AUTHORIZATION, StatusCode},
    response::Json,
};
use tracing::{debug, error, info, warn};

/// List available models handler for /api/llm/models
pub async fn list_models_handler(
    State(repo): State<LiteLLMRepository>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ModelsResponse>, (StatusCode, String)> {
    debug!("Handling list models request");

    // Validate authentication
    let auth_header = headers.get(AUTHORIZATION)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    let token = auth_header.to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Authorization header".to_string()))?;

    if !token.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "Invalid Bearer token format".to_string()));
    }

    // Process request
    match repo.list_models().await {
        Ok(litellm_response) => {
            let backend_response = repo.convert_models_response(litellm_response);
            info!("Successfully listed {} models", backend_response.models.len());
            Ok(Json(backend_response))
        }
        Err(e) => {
            error!("Failed to list models: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list models".to_string()))
        }
    }
}

/// Chat completion handler for /api/llm/chat
pub async fn chat_handler(
    State(repo): State<LiteLLMRepository>,
    headers: axum::http::HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    debug!("Handling chat request for model: {}", request.model);

    // Validate authentication
    let auth_header = headers.get(AUTHORIZATION)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    let token = auth_header.to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Authorization header".to_string()))?;

    if !token.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "Invalid Bearer token format".to_string()));
    }

    // Process request
    let litellm_request = repo.convert_chat_request(request);

    match repo.create_chat_completion(litellm_request).await {
        Ok(litellm_response) => {
            let backend_response = repo.convert_chat_response(litellm_response);
            info!("Successfully processed chat request with model: {}", backend_response.model);
            Ok(Json(backend_response))
        }
        Err(e) => {
            error!("Failed to create chat completion: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create chat completion".to_string()))
        }
    }
}

/// Health check handler for LiteLLM service
pub async fn health_check_handler(
    State(repo): State<LiteLLMRepository>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    debug!("Handling LiteLLM health check");

    match repo.health_check().await {
        Ok(is_healthy) => {
            let status = if is_healthy {
                info!("LiteLLM health check passed");
                Json(serde_json::json!({
                    "status": "healthy",
                    "service": "litellm",
                    "socket_path": repo.socket_path()
                }))
            } else {
                warn!("LiteLLM health check failed");
                Json(serde_json::json!({
                    "status": "unhealthy",
                    "service": "litellm",
                    "socket_path": repo.socket_path()
                }))
            };
            Ok(status)
        }
        Err(e) => {
            error!("LiteLLM health check error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}