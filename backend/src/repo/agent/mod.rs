use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use futures::StreamExt;
use reqwest::Client;
use tokio::fs;
use uuid::Uuid;
use serde_json::Value;

use crate::model::agent::{
    AgentConfig, AgentEvent, AgentResult, AgentError,
    ChatMessage, ChatRequest, ToolCall, ToolFunction, EventMetadata,
};

pub mod events;
pub mod session;
pub mod tools;

pub use events::{EventBroadcaster, EventSubscription};
pub use session::{SessionManager, SessionInfo};
pub use tools::{ToolRegistry, AgentTool};

/// Main repository for agent operations
pub struct AgentRepo {
    config: AgentConfig,
    http_client: Client,
    tool_registry: ToolRegistry,
    session_manager: Arc<SessionManager>,
    event_broadcaster: Arc<EventBroadcaster>,
    workspace_path: PathBuf,
}

/// Streaming context for processing SSE chunks
#[derive(Debug)]
struct StreamingContext {
    content_buffer: String,
    tool_calls: Vec<ToolCall>,
    current_tool: Option<PartialToolCall>,
    args_buffer: String,
    depth: i32,
}

#[derive(Debug)]
struct PartialToolCall {
    id: String,
    name: String,
}

impl StreamingContext {
    fn new() -> Self {
        Self {
            content_buffer: String::new(),
            tool_calls: Vec::new(),
            current_tool: None,
            args_buffer: String::new(),
            depth: 0,
        }
    }

    fn process_chunk(&mut self, data: &Value) -> Vec<StreamingEvent> {
        let mut events = Vec::new();

        if let Some(choices) = data.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    // Handle content chunks
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        self.content_buffer.push_str(content);
                        events.push(StreamingEvent::Content(content.to_string()));
                    }

                    // Handle tool call chunks
                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                        for tool_call in tool_calls {
                            // Tool call start
                            if let Some(id) = tool_call.get("id").and_then(|i| i.as_str()) {
                                if let Some(function) = tool_call.get("function") {
                                    if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                        self.current_tool = Some(PartialToolCall {
                                            id: id.to_string(),
                                            name: name.to_string(),
                                        });
                                        events.push(StreamingEvent::ToolCallStart {
                                            id: id.to_string(),
                                            name: name.to_string(),
                                        });
                                    }
                                }
                            }

                            // Tool call arguments
                            if let Some(function) = tool_call.get("function") {
                                if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                                    self.args_buffer.push_str(args);
                                    
                                    // Track JSON depth
                                    for ch in args.chars() {
                                        match ch {
                                            '{' | '[' => self.depth += 1,
                                            '}' | ']' => {
                                                self.depth -= 1;
                                                if self.depth == 0 && !self.args_buffer.is_empty() {
                                                    // Complete tool call
                                                    if let Some(tool) = self.current_tool.take() {
                                                        let tool_call = ToolCall {
                                                            id: tool.id,
                                                            call_type: "function".to_string(),
                                                            function: ToolFunction {
                                                                name: tool.name,
                                                                arguments: self.args_buffer.clone(),
                                                            },
                                                        };
                                                        self.tool_calls.push(tool_call.clone());
                                                        events.push(StreamingEvent::ToolCallReady(tool_call));
                                                        self.args_buffer.clear();
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        events
    }

    fn finalize(self) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: if self.content_buffer.is_empty() {
                None
            } else {
                Some(self.content_buffer)
            },
            tool_calls: if self.tool_calls.is_empty() {
                None
            } else {
                Some(self.tool_calls)
            },
            tool_call_id: None,
        }
    }
}

#[derive(Debug)]
enum StreamingEvent {
    Content(String),
    ToolCallStart { id: String, name: String },
    ToolCallReady(ToolCall),
}

impl AgentRepo {

    /// Creates metadata for a new operation
    fn create_metadata(operation_id: &str) -> EventMetadata {
        EventMetadata::new(operation_id.to_string())
    }

    /// Creates metadata for file operations with path extraction
    fn create_file_metadata(operation_id: &str, tool: &str, result: &str) -> EventMetadata {
        let is_file_op = Self::is_file_modifying_tool(tool);
        let paths = if is_file_op {
            Self::extract_paths_from_result(result)
        } else {
            Vec::new()
        };
        
        EventMetadata::new(operation_id.to_string())
            .with_file_operation(is_file_op)
            .with_file_paths(paths)
    }

    /// Determines if a tool modifies files (excludes read_file)
    fn is_file_modifying_tool(tool: &str) -> bool {
        matches!(
            tool,
            "write_file" | "update_file" | "create_file" | "delete_file" | "create_directory"
        )
    }

    /// Extracts file paths from tool result
    fn extract_paths_from_result(result: &str) -> Vec<String> {
        // Try JSON parsing first
        if let Ok(json_result) = serde_json::from_str::<serde_json::Value>(result) {
            if let Some(path) = json_result.get("path").and_then(|p| p.as_str()) {
                return vec![path.to_string()];
            }
        }

        // Fallback to regex pattern matching
        use regex::Regex;
        let patterns = [
            r#"Successfully (?:wrote|created|updated|deleted) (?:file |directory )?["']([^"']+)["']"#,
            r#"["']([^"']*\.[a-zA-Z0-9]+)["']"#, // File with extension in quotes
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(result) {
                    if let Some(path) = captures.get(1) {
                        return vec![path.as_str().to_string()];
                    }
                }
            }
        }

        Vec::new()
    }
    /// Creates a new agent repository
    pub async fn new(
        workspace_path: PathBuf,
        litellm_base_url: String,
    ) -> AgentResult<Self> {
        // Load system prompt from file
        let system_prompt = Self::load_system_prompt(&workspace_path).await?;
        
        let config = AgentConfig {
            litellm_base_url,
            system_prompt,
            max_session_age: Duration::from_secs(3600), // 1 hour
        };
        
        let http_client = Client::new();
        let tool_registry = ToolRegistry::new();
        let session_manager = Arc::new(SessionManager::new(config.max_session_age));
        let event_broadcaster = Arc::new(EventBroadcaster::new());
        
        // Start background cleanup task
        let cleanup_handle = session_manager.clone().start_cleanup_task();
        tokio::spawn(cleanup_handle);
        
        Ok(Self {
            config,
            http_client,
            tool_registry,
            session_manager,
            event_broadcaster,
            workspace_path,
        })
    }
    
    /// Loads the system prompt from the config file
    async fn load_system_prompt(workspace_path: &PathBuf) -> AgentResult<String> {
        let prompt_path = workspace_path.parent()
            .unwrap_or(workspace_path)
            .join("config")
            .join("systemprompt.md");
        
        match fs::read_to_string(&prompt_path).await {
            Ok(content) => Ok(content),
            Err(_) => {
                tracing::warn!("Could not load system prompt from {:?}, using default", prompt_path);
                Ok("You are a helpful LaTeX document assistant.".to_string())
            }
        }
    }
    
    /// Processes a chat request with tool calling support
    pub async fn process_chat(&self, request: ChatRequest) -> AgentResult<String> {
        let job_id = Uuid::new_v4().to_string();
        
        // Notify job queued
        self.event_broadcaster
            .broadcast(&request.session_id, AgentEvent::JobQueued { 
                job_id: job_id.clone(), 
                session_id: request.session_id.clone(),
                metadata: Self::create_metadata(&job_id),
            })
            .await;
        
        // Get or create session
        let _session = self.session_manager
            .get_or_create_session(request.session_id.clone(), None)
            .await;
        
        // Get session history
        let mut messages = self.session_manager
            .get_history(&request.session_id)
            .await?;
        
        // Add system prompt if this is the first message
        if messages.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(self.config.system_prompt.clone()),
                ..Default::default()
            });
        }
        
        // Add user message
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(request.message),
            ..Default::default()
        };
        
        messages.push(user_message.clone());
        self.session_manager
            .add_message(&request.session_id, user_message)
            .await?;
        
        // Process with tool calling loop
        let response = self.process_with_tools(messages, request.model, &request.session_id).await?;
        
        // Add assistant response to session
        self.session_manager
            .add_message(&request.session_id, response.clone())
            .await?;
        
        let content = response.content.unwrap_or_default();
        
        // Notify completion
        self.event_broadcaster
            .broadcast(&request.session_id, AgentEvent::JobComplete { 
                response: content.clone(),
                metadata: Self::create_metadata(&job_id),
            })
            .await;
        
        Ok(content)
    }
    
    /// Processes messages with tool calling loop
    async fn process_with_tools(
        &self,
        mut messages: Vec<ChatMessage>,
        model: String,
        session_id: &str,
    ) -> AgentResult<ChatMessage> {
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 10; // Prevent infinite loops
        
        loop {
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                return Err(AgentError::Generic(anyhow::anyhow!(
                    "Maximum iteration count reached in tool calling loop"
                )));
            }
            
            // Call LiteLLM
            self.event_broadcaster
                .broadcast(session_id, AgentEvent::LLMCallStart { 
                    model: model.clone(),
                    metadata: Self::create_metadata(&format!("llm-{}", iteration_count)),
                })
                .await;
            
            let response = self.call_litellm(&messages, &model, session_id).await?;
            
            // Check for tool calls
            if let Some(tool_calls) = response.tool_calls.clone() {
                if tool_calls.is_empty() {
                    // No tool calls - return response directly
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::LLMCallComplete {
                            metadata: Self::create_metadata(&format!("llm-complete-{}", iteration_count)),
                        })
                        .await;
                    return Ok(response);
                }
                
                // Add assistant message with tool calls to history FIRST
                messages.push(response);
                
                if tool_calls.len() > 1 {
                    // Multiple tools - execute in parallel
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::ParallelToolsStart { 
                            count: tool_calls.len(),
                            metadata: Self::create_metadata(&format!("parallel-tools-{}", iteration_count)),
                        })
                        .await;
                    
                    // Create futures for all tool executions
                    let tool_futures: Vec<_> = tool_calls
                        .iter()
                        .map(|tool_call| self.execute_tool_async(tool_call.clone(), session_id))
                        .collect();
                    
                    // Execute all tools in parallel
                    let results = join_all(tool_futures).await;
                    
                    // Process results and add to messages
                    for (tool_call, result) in tool_calls.iter().zip(results.iter()) {
                        match result {
                            Ok(tool_result) => {
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(tool_result.clone()),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                                
                                self.event_broadcaster
                                    .broadcast(session_id, AgentEvent::ToolCompleted {
                                        tool: tool_call.function.name.clone(),
                                        result: tool_result.clone(),
                                        metadata: Self::create_file_metadata(&format!("tool-{}", tool_call.id), &tool_call.function.name, tool_result),
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let error_msg = format!("Tool execution failed: {}", e);
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(error_msg.clone()),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                                
                                self.event_broadcaster
                                    .broadcast(session_id, AgentEvent::Error {
                                        message: error_msg,
                                        metadata: Self::create_metadata(&format!("error-{}", iteration_count)),
                                    })
                                    .await;
                            }
                        }
                    }
                    
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::ParallelToolsComplete { 
                            count: tool_calls.len(),
                            metadata: Self::create_metadata(&format!("parallel-complete-{}", iteration_count)),
                        })
                        .await;
                    
                } else {
                    // Single tool - execute normally
                    for tool_call in tool_calls {
                        self.event_broadcaster
                            .broadcast(session_id, AgentEvent::ToolCallRequested {
                                tool: tool_call.function.name.clone(),
                                args: serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or(serde_json::Value::Null),
                                metadata: Self::create_metadata(&format!("tool-req-{}", tool_call.id)),
                            })
                            .await;
                        
                        match self.execute_tool(&tool_call.function).await {
                            Ok(result) => {
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(result.clone()),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                                
                                self.event_broadcaster
                                    .broadcast(session_id, AgentEvent::ToolCompleted {
                                        tool: tool_call.function.name.clone(),
                                        result: result.clone(),
                                        metadata: Self::create_file_metadata(&format!("tool-{}", tool_call.id), &tool_call.function.name, &result),
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let error_msg = format!("Tool execution failed: {}", e);
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(error_msg.clone()),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                                
                                self.event_broadcaster
                                    .broadcast(session_id, AgentEvent::Error {
                                        message: error_msg,
                                        metadata: Self::create_metadata(&format!("error-{}", iteration_count)),
                                    })
                                    .await;
                            }
                        }
                    }
                }
                
                // Continue loop for next LLM call with tool results
            } else {
                // No tool calls, return final response
                self.event_broadcaster
                    .broadcast(session_id, AgentEvent::LLMCallComplete {
                        metadata: Self::create_metadata(&format!("llm-complete-{}", iteration_count)),
                    })
                    .await;
                return Ok(response);
            }
        }
    }
    
    /// Executes a tool asynchronously (for parallel execution)
    async fn execute_tool_async(&self, tool_call: ToolCall, session_id: &str) -> AgentResult<String> {
        self.event_broadcaster
            .broadcast(session_id, AgentEvent::ToolExecuting {
                tool: tool_call.function.name.clone(),
                metadata: Self::create_metadata(&format!("tool-exec-{}", tool_call.id)),
            })
            .await;
        
        self.execute_tool(&tool_call.function).await
    }
    
    /// Executes a single tool
    async fn execute_tool(&self, tool_function: &ToolFunction) -> AgentResult<String> {
        let args: serde_json::Value = serde_json::from_str(&tool_function.arguments)
            .map_err(|e| AgentError::InvalidToolArguments {
                tool: tool_function.name.clone(),
                error: format!("Invalid JSON arguments: {}", e),
            })?;
        
        self.tool_registry
            .execute(&tool_function.name, &self.workspace_path, args)
            .await
    }
    
    /// Calls LiteLLM API with streaming support
    async fn call_litellm(&self, messages: &[ChatMessage], model: &str, session_id: &str) -> AgentResult<ChatMessage> {
        // Build request with streaming enabled
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": self.tool_registry.get_definitions(),
            "tool_choice": "auto",
            "stream": true
        });
        
        let response = self.http_client
            .post(format!("{}/v1/chat/completions", self.config.litellm_base_url))
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(AgentError::HttpError)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::LiteLLMError {
                message: format!("HTTP {}: {}", status, error_text),
            });
        }
        
        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut context = StreamingContext::new();
        let mut buffer = String::new();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result.map_err(AgentError::HttpError)?;
            let chunk_str = String::from_utf8_lossy(&chunk_bytes);
            buffer.push_str(&chunk_str);
            
            // Process complete lines
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer.drain(..=line_end).collect::<String>();
                let line = line.trim();
                
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                
                // Parse SSE data
                if line.starts_with("data: ") {
                    let data_str = &line[6..];
                    
                    // Check for stream end
                    if data_str == "[DONE]" {
                        break;
                    }
                    
                    // Parse JSON chunk
                    if let Ok(data) = serde_json::from_str::<Value>(data_str) {
                        let events = context.process_chunk(&data);
                        
                        // Emit events
                        for event in events {
                            match event {
                                StreamingEvent::Content(content) => {
                                    self.event_broadcaster
                                        .broadcast(session_id, AgentEvent::StreamChunk {
                                            content,
                                            metadata: Self::create_metadata("stream-chunk"),
                                        })
                                        .await;
                                }
                                StreamingEvent::ToolCallStart { id, name } => {
                                    self.event_broadcaster
                                        .broadcast(session_id, AgentEvent::ToolCallStart {
                                            tool_id: id,
                                            tool_name: name,
                                            metadata: Self::create_metadata("tool-start"),
                                        })
                                        .await;
                                }
                                StreamingEvent::ToolCallReady(tool_call) => {
                                    self.event_broadcaster
                                        .broadcast(session_id, AgentEvent::ToolCallReady {
                                            tool_call,
                                            metadata: Self::create_metadata("tool-ready"),
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Return the finalized message
        Ok(context.finalize())
    }
    
    /// Gets the event broadcaster for WebSocket integration
    pub fn get_event_broadcaster(&self) -> Arc<EventBroadcaster> {
        self.event_broadcaster.clone()
    }
    
    /// Gets the session manager
    pub fn get_session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }
    
    /// Gets available tool definitions
    pub fn get_tool_definitions(&self) -> Vec<crate::model::agent::ToolDefinition> {
        self.tool_registry.get_definitions()
    }
    
    /// Gets workspace path
    pub fn get_workspace_path(&self) -> &PathBuf {
        &self.workspace_path
    }
}