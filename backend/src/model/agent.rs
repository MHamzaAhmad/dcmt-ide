use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

// Request/Response types for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
    pub model: String, // e.g., "gpt-4-turbo"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub message: String,
    pub job_id: String,
}

// Structured response from LLM (when parsed from JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredAgentResponse {
    pub message: String,
    pub reasoning: String,
    pub actions: Vec<String>,
    pub files_modified: Vec<String>,
    pub suggestions: Vec<String>,
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
    pub parameters: Value,
}

// WebSocket event types for real-time progress updates
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    JobQueued {
        job_id: String,
        session_id: String,
    },
    LLMCallStart {
        model: String,
    },
    LLMStreaming {
        content: String,
    },
    ToolCallRequested {
        tool: String,
        args: Value,
    },
    ToolExecuting {
        tool: String,
    },
    ToolCompleted {
        tool: String,
        result: String,
    },
    ParallelToolsStart {
        count: usize,
    },
    ParallelToolsComplete {
        count: usize,
    },
    LLMCallComplete,
    JobComplete {
        response: String,
    },
    Error {
        message: String,
    },
}

// Session management structures
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub user_id: Option<String>, // For future multi-user support
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

    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn is_expired(&self, max_age: std::time::Duration) -> bool {
        self.last_activity.elapsed() > max_age
    }
}

// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub litellm_base_url: String,
    pub system_prompt: String,
    pub max_session_age: std::time::Duration,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            litellm_base_url: "http://0.0.0.0:4000".to_string(),
            system_prompt: String::new(),
            max_session_age: std::time::Duration::from_secs(3600), // 1 hour
        }
    }
}

// Error types for agent operations
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },
    
    #[error("Tool execution failed: {tool}: {error}")]
    ToolExecutionError { tool: String, error: String },
    
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },
    
    #[error("Invalid tool arguments for {tool}: {error}")]
    InvalidToolArguments { tool: String, error: String },
    
    #[error("LiteLLM API error: {message}")]
    LiteLLMError { message: String },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

// Result type alias for agent operations
pub type AgentResult<T> = std::result::Result<T, AgentError>;