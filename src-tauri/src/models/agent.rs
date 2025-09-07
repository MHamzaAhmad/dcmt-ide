use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};

// Request/Response types for Tauri commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
    pub model: String, // e.g., "gpt-4-turbo"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub job_id: String,
}

// LiteLLM API structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: String, // "auto"
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String, // "json_object"
}

// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            role: "user".to_string(),
            content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

// Tool call structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String, // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String, // JSON string
}

// Tool definition structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String, // "function"
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON schema
}

// Agent events for real-time progress updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    JobQueued { job_id: String, session_id: String },
    LLMCallStart { model: String },
    LLMStreaming { content: String },
    ToolExecuting { tool: String },
    ToolCompleted { tool: String, result: String },
    LLMCallComplete,
    JobComplete { response: String },
    Error { message: String },
}

// Session management
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub user_id: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub created_at: Instant,
    pub last_activity: Instant,
}

impl ChatSession {
    pub fn new(id: String, user_id: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            id,
            user_id,
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
        }
    }
    
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.last_activity = Instant::now();
    }
    
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub user_id: Option<String>,
    pub message_count: usize,
    pub created_at: String,
    pub last_activity: String,
}

// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub litellm_base_url: String,
    pub system_prompt: String,
    pub max_session_age: Duration,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("LiteLLM API error: {message}")]
    LiteLLMError { message: String },
    
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },
    
    #[error("Invalid tool arguments for {tool}: {error}")]
    InvalidToolArguments { tool: String, error: String },
    
    #[error("Tool execution failed for {tool}: {error}")]
    ToolExecutionError { tool: String, error: String },
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

pub type AgentResult<T> = Result<T, AgentError>;