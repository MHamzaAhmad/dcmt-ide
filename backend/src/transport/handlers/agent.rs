use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info};

use crate::model::agent::{ChatRequest, ChatResponse, ToolDefinition};
use crate::repo::agent::SessionInfo;
use crate::svc::AgentService;

/// Main chat handler - queues agent requests for processing
pub async fn chat_handler(
    State(service): State<Arc<AgentService>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    info!(
        "Received chat request for session: {}, model: {}, message length: {}", 
        request.session_id, 
        request.model, 
        request.message.len()
    );
    
    // Validate request
    if request.session_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST, 
            "session_id is required".to_string()
        ));
    }
    
    if request.message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST, 
            "message cannot be empty".to_string()
        ));
    }
    
    if request.model.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST, 
            "model is required".to_string()
        ));
    }
    
    // Queue the chat request
    match service.queue_chat(request).await {
        Ok(response) => {
            info!("Chat request queued successfully: job_id={}", response.job_id);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to queue chat request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to process request: {}", e),
            ))
        }
    }
}

/// Returns information about available tools
pub async fn list_tools_handler(
    State(service): State<Arc<AgentService>>,
) -> Result<Json<ToolsResponse>, (StatusCode, String)> {
    let tools = service.get_available_tools();
    let count = tools.len();
    
    Ok(Json(ToolsResponse {
        tools,
        count,
    }))
}

/// Returns information about a specific session
pub async fn session_info_handler(
    State(service): State<Arc<AgentService>>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionInfoResponse>, (StatusCode, String)> {
    if let Some(session_info) = service.get_session_info(&session_id).await {
        Ok(Json(SessionInfoResponse {
            session: session_info,
            found: true,
        }))
    } else {
        Ok(Json(SessionInfoResponse {
            session: SessionInfo {
                id: session_id,
                user_id: None,
                message_count: 0,
                created_at: std::time::Instant::now(),
                last_activity: std::time::Instant::now(),
            },
            found: false,
        }))
    }
}

// Response types for API endpoints

#[derive(serde::Serialize)]
pub struct ToolsResponse {
    pub tools: Vec<ToolDefinition>,
    pub count: usize,
}

#[derive(serde::Serialize)]
pub struct SessionInfoResponse {
    pub session: SessionInfo,
    pub found: bool,
}