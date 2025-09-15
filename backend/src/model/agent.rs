use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

// Request/Response types for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
    pub model: String, // e.g., "gpt-4-turbo"
    pub auth_token: Option<String>, // JWT token for LiteLLM authentication
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>, // Human-readable name e.g. "List Files"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progressive_form: Option<String>, // Progressive form e.g. "Listing files"
}

// WebSocket event types for real-time progress updates
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    JobQueued {
        job_id: String,
        session_id: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    LLMCallStart {
        model: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    LLMStreaming {
        content: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    StreamChunk {
        content: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ToolCallStart {
        tool_id: String,
        tool_name: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ToolCallReady {
        tool_call: ToolCall,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ToolCallRequested {
        tool: String,
        args: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progressive_form: Option<String>,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ToolExecuting {
        tool: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progressive_form: Option<String>,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ToolCompleted {
        tool: String,
        result: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ParallelToolsStart {
        count: usize,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    ParallelToolsComplete {
        count: usize,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    LLMCallComplete {
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    JobComplete {
        response: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
    Error {
        message: String,
        #[serde(flatten)]
        metadata: EventMetadata,
    },
}

// Event metadata for deduplication and tracing
#[derive(Debug, Clone, Serialize)]
pub struct EventMetadata {
    pub event_id: String,
    pub operation_id: String,
    pub timestamp: u64,
    pub is_file_operation: bool,
    pub file_paths: Vec<String>, // Extracted file paths if applicable
}

impl EventMetadata {
    pub fn new(operation_id: String) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            operation_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            is_file_operation: false,
            file_paths: Vec::new(),
        }
    }

    pub fn with_file_operation(mut self, is_file_op: bool) -> Self {
        self.is_file_operation = is_file_op;
        self
    }

    pub fn with_file_paths(mut self, paths: Vec<String>) -> Self {
        self.is_file_operation = !paths.is_empty();
        self.file_paths = paths;
        self
    }
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
    pub system_prompt: String,
    pub max_session_age: std::time::Duration,
    pub tavily_api_key: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            max_session_age: std::time::Duration::from_secs(3600), // 1 hour
            tavily_api_key: None,
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