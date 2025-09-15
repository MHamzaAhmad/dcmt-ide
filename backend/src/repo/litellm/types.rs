use serde::{Deserialize, Serialize};

/// LiteLLM model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

/// LiteLLM models list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMModelsResponse {
    pub object: String,
    pub data: Vec<LiteLLMModel>,
}

/// Chat message for LiteLLM API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<serde_json::Value>>,
    pub tool_choice: Option<serde_json::Value>,
}

/// Chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// Backend API response for models endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<LLMModel>,
}

/// Simplified model information for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMModel {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Backend API chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

/// Backend API chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub message: ChatMessage,
    pub usage: Option<Usage>,
}

#[derive(Debug, thiserror::Error)]
pub enum LiteLLMError {
    #[error("UDS connection failed: {0}")]
    UdsConnection(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("JSON serialization failed: {0}")]
    Serialization(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}