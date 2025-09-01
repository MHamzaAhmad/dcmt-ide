use axum::{
    extract::{Query, State},
    response::{Response, IntoResponse, sse::{Event, Sse}},
    http::{StatusCode, HeaderMap, header},
    routing::{get, post},
    Router, Json,
};
use serde::{Deserialize, Serialize};
use tokio_stream::{wrappers::BroadcastStream, Stream};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;
use std::{collections::HashMap, sync::Arc, time::Duration};
use anyhow::Result;
use tracing::{info, error, debug};
use futures::stream::{self, StreamExt};

// All types are defined in this file for simplicity

// All types are defined in this file

/// SSE message types for AI streaming
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SSEMessage {
    // AI Model responses
    ModelResponse {
        conversation_id: Uuid,
        model_id: String,
        response: ModelResponse,
    },
    
    // Model status updates
    ModelStatus {
        model_id: String,
        status: ModelStatus,
        tokens_per_second: Option<f32>,
        cost_estimate: Option<f64>,
    },
    
    // LaTeX compilation streaming
    CompilationProgress {
        document_id: Uuid,
        stage: CompilationStage,
        progress: f32,
        message: String,
    },
    
    // Error messages
    Error {
        error_id: Uuid,
        message: String,
        details: Option<String>,
    },
    
    // Heartbeat
    Heartbeat {
        timestamp: i64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelStatus {
    Loading,
    Ready,
    Busy,
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompilationStage {
    Parsing,
    PackageInstallation,
    Compiling,
    Completed,
    Failed,
}

/// AI Model request/response types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRequest {
    pub conversation_id: Uuid,
    pub model_id: String,
    pub prompt: String,
    pub context: Option<LaTeXContext>,
    pub parameters: Option<ModelParameters>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub is_complete: bool,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaTeXContext {
    pub document_content: String,
    pub current_position: Option<u32>,
    pub selected_text: Option<String>,
    pub packages: Vec<String>,
    pub document_class: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelParameters {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
}

/// SSE Handler for AI streaming and compilation progress
pub struct SSEHandler {
    model_manager: Arc<ModelManager>,
    broadcast_sender: broadcast::Sender<SSEMessage>,
    active_streams: Arc<tokio::sync::RwLock<HashMap<Uuid, mpsc::Sender<SSEMessage>>>>,
}

impl SSEHandler {
    pub fn new(model_manager: Arc<ModelManager>) -> (Self, broadcast::Receiver<SSEMessage>) {
        let (broadcast_sender, broadcast_receiver) = broadcast::channel(1000);
        
        let handler = Self {
            model_manager,
            broadcast_sender,
            active_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };
        
        (handler, broadcast_receiver)
    }
    
    pub fn router(self) -> Router {
        let app_state = Arc::new(self);
        
        Router::new()
            .route("/stream", get(sse_stream_handler))
            .route("/chat", post(ai_chat_handler))
            .route("/models", get(list_models_handler))
            .route("/models/:model_id/status", get(model_status_handler))
            .with_state(app_state)
    }
    
    pub async fn send_message(&self, message: SSEMessage) -> Result<()> {
        let _ = self.broadcast_sender.send(message);
        Ok(())
    }
    
    pub async fn stream_ai_response(
        &self,
        request: ModelRequest,
        stream_id: Uuid,
    ) -> Result<()> {
        let model_manager = Arc::clone(&self.model_manager);
        let broadcast_sender = self.broadcast_sender.clone();
        let conversation_id = request.conversation_id;
        let model_id = request.model_id.clone();
        
        tokio::spawn(async move {
            debug!("Starting AI response stream for conversation {}", conversation_id);
            
            // Update model status
            let _ = broadcast_sender.send(SSEMessage::ModelStatus {
                model_id: model_id.clone(),
                status: ModelStatus::Busy,
                tokens_per_second: None,
                cost_estimate: None,
            });
            
            // Stream the response
            match model_manager.stream_response(request).await {
                Ok(mut response_stream) => {
                    while let Some(response) = response_stream.next().await {
                        let message = SSEMessage::ModelResponse {
                            conversation_id,
                            model_id: model_id.clone(),
                            response,
                        };
                        
                        let _ = broadcast_sender.send(message);
                    }
                }
                Err(e) => {
                    error!("AI streaming error: {}", e);
                    let _ = broadcast_sender.send(SSEMessage::Error {
                        error_id: Uuid::new_v4(),
                        message: format!("AI model error: {}", e),
                        details: None,
                    });
                }
            }
            
            // Update model status back to ready
            let _ = broadcast_sender.send(SSEMessage::ModelStatus {
                model_id,
                status: ModelStatus::Ready,
                tokens_per_second: None,
                cost_estimate: None,
            });
        });
        
        Ok(())
    }
    
    pub async fn stream_compilation_progress(
        &self,
        document_id: Uuid,
        progress_receiver: mpsc::Receiver<(CompilationStage, f32, String)>,
    ) {
        let broadcast_sender = self.broadcast_sender.clone();
        let mut receiver = progress_receiver;
        
        tokio::spawn(async move {
            while let Some((stage, progress, message)) = receiver.recv().await {
                let sse_message = SSEMessage::CompilationProgress {
                    document_id,
                    stage,
                    progress,
                    message,
                };
                
                let _ = broadcast_sender.send(sse_message);
            }
        });
    }
    
    async fn start_heartbeat(&self) {
        let sender = self.broadcast_sender.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let heartbeat = SSEMessage::Heartbeat {
                    timestamp: chrono::Utc::now().timestamp(),
                };
                
                let _ = sender.send(heartbeat);
            }
        });
    }
}

/// SSE stream handler
async fn sse_stream_handler(
    Query(params): Query<StreamParams>,
    State(handler): State<Arc<SSEHandler>>,
) -> impl IntoResponse {
    let stream_id = Uuid::new_v4();
    let receiver = handler.broadcast_sender.subscribe();
    
    info!("New SSE stream connected: {}", stream_id);
    
    // Convert broadcast receiver to SSE stream
    let stream = BroadcastStream::new(receiver)
        .filter_map(|msg| async move {
            match msg {
                Ok(sse_msg) => {
                    let json = serde_json::to_string(&sse_msg).ok()?;
                    Some(Event::default().data(json))
                }
                Err(e) => {
                    error!("SSE broadcast error: {}", e);
                    None
                }
            }
        })
        .map(Ok::<_, axum::Error>);
    
    // Set up SSE response headers
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());
    headers.insert("X-Accel-Buffering", "no".parse().unwrap());
    
    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

#[derive(Debug, Deserialize)]
struct StreamParams {
    #[serde(default)]
    document_id: Option<Uuid>,
    #[serde(default)]
    conversation_id: Option<Uuid>,
}

/// AI Chat handler
async fn ai_chat_handler(
    State(handler): State<Arc<SSEHandler>>,
    Json(request): Json<ModelRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let stream_id = Uuid::new_v4();
    
    handler
        .stream_ai_response(request, stream_id)
        .await
        .map_err(|e| {
            error!("AI chat error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(serde_json::json!({
        "stream_id": stream_id,
        "status": "streaming"
    })))
}

/// List available models
async fn list_models_handler(
    State(handler): State<Arc<SSEHandler>>,
) -> impl IntoResponse {
    let models = handler.model_manager.list_available_models().await;
    Json(models)
}

/// Get model status
async fn model_status_handler(
    axum::extract::Path(model_id): axum::extract::Path<String>,
    State(handler): State<Arc<SSEHandler>>,
) -> impl IntoResponse {
    match handler.model_manager.get_model_status(&model_id).await {
        Ok(status) => Json(serde_json::json!({
            "model_id": model_id,
            "status": status
        })),
        Err(e) => {
            error!("Error getting model status: {}", e);
            Json(serde_json::json!({
                "error": format!("Model not found: {}", e)
            }))
        }
    }
}

/// Model Manager for handling different AI models
pub struct ModelManager {
    local_models: HashMap<String, Box<dyn LocalModel + Send + Sync>>,
    remote_models: HashMap<String, Box<dyn RemoteModel + Send + Sync>>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            local_models: HashMap::new(),
            remote_models: HashMap::new(),
        }
    }
    
    pub async fn add_local_model(&mut self, model_id: String, model: Box<dyn LocalModel + Send + Sync>) {
        self.local_models.insert(model_id, model);
    }
    
    pub async fn add_remote_model(&mut self, model_id: String, model: Box<dyn RemoteModel + Send + Sync>) {
        self.remote_models.insert(model_id, model);
    }
    
    pub async fn stream_response(
        &self,
        request: ModelRequest,
    ) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>> {
        let model_id = &request.model_id;
        
        // Check local models first
        if let Some(local_model) = self.local_models.get(model_id) {
            return local_model.stream_response(request).await;
        }
        
        // Then check remote models
        if let Some(remote_model) = self.remote_models.get(model_id) {
            return remote_model.stream_response(request).await;
        }
        
        Err(anyhow::anyhow!("Model not found: {}", model_id))
    }
    
    pub async fn list_available_models(&self) -> Vec<ModelInfo> {
        let mut models = Vec::new();
        
        for (id, model) in &self.local_models {
            models.push(ModelInfo {
                id: id.clone(),
                name: model.name(),
                model_type: ModelType::Local,
                status: model.status().await,
            });
        }
        
        for (id, model) in &self.remote_models {
            models.push(ModelInfo {
                id: id.clone(),
                name: model.name(),
                model_type: ModelType::Remote,
                status: model.status().await,
            });
        }
        
        models
    }
    
    pub async fn get_model_status(&self, model_id: &str) -> Result<ModelStatus> {
        if let Some(model) = self.local_models.get(model_id) {
            return Ok(model.status().await);
        }
        
        if let Some(model) = self.remote_models.get(model_id) {
            return Ok(model.status().await);
        }
        
        Err(anyhow::anyhow!("Model not found: {}", model_id))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub model_type: ModelType,
    pub status: ModelStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelType {
    Local,
    Remote,
}

#[async_trait::async_trait]
pub trait LocalModel {
    fn name(&self) -> String;
    async fn status(&self) -> ModelStatus;
    async fn stream_response(
        &self,
        request: ModelRequest,
    ) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>>;
}

#[async_trait::async_trait]
pub trait RemoteModel {
    fn name(&self) -> String;
    async fn status(&self) -> ModelStatus;
    async fn stream_response(
        &self,
        request: ModelRequest,
    ) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sse_message_serialization() {
        let message = SSEMessage::ModelResponse {
            conversation_id: Uuid::new_v4(),
            model_id: "test-model".to_string(),
            response: ModelResponse {
                content: "Hello world".to_string(),
                is_complete: false,
                tokens_used: Some(10),
                finish_reason: None,
            },
        };
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("ModelResponse"));
        assert!(json.contains("Hello world"));
    }
}