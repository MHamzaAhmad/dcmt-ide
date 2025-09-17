use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use futures::StreamExt;
use reqwest::Client;
use tokio::fs;
use uuid::Uuid;

use crate::models::agent::{
    AgentConfig, AgentEvent, AgentResult, AgentError,
    ChatMessage, ChatRequest, ChatResponse, 
    ToolCall, ToolFunction, EventMetadata,
};

use super::agent_session::SessionManager;
use super::agent_events::EventBroadcaster;
use super::agent_tools::ToolRegistry;

/// Main service for agent operations with LiteLLM integration and parallel processing
pub struct AgentService {
    config: AgentConfig,
    http_client: Client,
    tool_registry: ToolRegistry,
    session_manager: Arc<SessionManager>,
    event_broadcaster: Arc<EventBroadcaster>,
    workspace_path: Option<PathBuf>, // None until project is selected
    app_handle: tauri::AppHandle,
}

/// Context for streaming SSE chunks and building complete tool calls
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
    args: String,
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

    fn process_chunk(&mut self, line: &str) -> Result<Option<StreamChunk>, serde_json::Error> {
        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(line) {
            if let Some(delta) = &chunk.choices.first().and_then(|c| c.delta.as_ref()) {
                if let Some(content) = &delta.content {
                    self.content_buffer.push_str(content);
                    return Ok(Some(chunk));
                }
                
                if let Some(tool_calls) = &delta.tool_calls {
                    for tool_call_delta in tool_calls {
                        self.process_tool_call_delta(tool_call_delta);
                    }
                    return Ok(Some(chunk));
                }
            }
        }
        Ok(None)
    }

    fn process_tool_call_delta(&mut self, tool_call_delta: &ToolCallDelta) {
        if let Some(id) = &tool_call_delta.id {
            if let Some(_current) = &mut self.current_tool {
                self.finalize_current_tool();
            }
            
            self.current_tool = Some(PartialToolCall {
                id: id.clone(),
                name: tool_call_delta.function.name.clone().unwrap_or_default(),
                args: String::new(),
            });
            self.args_buffer.clear();
            self.depth = 0;
        }

        if let Some(current) = &mut self.current_tool {
            if let Some(args) = &tool_call_delta.function.arguments {
                self.args_buffer.push_str(args);
                
                for ch in args.chars() {
                    match ch {
                        '{' | '[' => self.depth += 1,
                        '}' | ']' => self.depth -= 1,
                        _ => {}
                    }
                }
                
                current.args = self.args_buffer.clone();
                
                if self.depth == 0 && !self.args_buffer.is_empty() {
                    self.finalize_current_tool();
                }
            }
        }
    }

    fn finalize_current_tool(&mut self) {
        if let Some(current) = self.current_tool.take() {
            if !current.args.is_empty() {
                let tool_call = ToolCall {
                    id: current.id,
                    call_type: "function".to_string(),
                    function: ToolFunction {
                        name: current.name,
                        arguments: current.args,
                    },
                };
                self.tool_calls.push(tool_call);
            }
        }
    }

    fn into_message(mut self) -> ChatMessage {
        self.finalize_current_tool();
        
        ChatMessage {
            role: "assistant".to_string(),
            content: if self.content_buffer.is_empty() { None } else { Some(self.content_buffer) },
            tool_calls: if self.tool_calls.is_empty() { None } else { Some(self.tool_calls) },
            tool_call_id: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, serde::Deserialize)]
struct ToolCallDelta {
    id: Option<String>,
    function: ToolFunctionDelta,
}

#[derive(Debug, serde::Deserialize)]
struct ToolFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

impl AgentService {

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
            "write_file" | "patch_file" | "create_file" | "delete_file" | "create_directory"
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
    /// Creates a new agent service
    pub async fn new(
        event_broadcaster: Arc<EventBroadcaster>,
        litellm_base_url: String,
        app_handle: tauri::AppHandle,
    ) -> AgentResult<Self> {
        // Load system prompt from embedded config or default
        let system_prompt = Self::load_system_prompt().await;
        
        let config = AgentConfig {
            litellm_base_url,
            system_prompt,
            max_session_age: Duration::from_secs(3600), // 1 hour
        };
        
        let http_client = Client::new();
        let tool_registry = ToolRegistry::new();
        let session_manager = Arc::new(SessionManager::new(config.max_session_age));
        
        // Start background cleanup task
        let cleanup_handle = session_manager.clone().start_cleanup_task();
        tokio::spawn(cleanup_handle);
        
        Ok(Self {
            config,
            http_client,
            tool_registry,
            session_manager,
            event_broadcaster,
            workspace_path: None,
            app_handle,
        })
    }
    
    /// Sets the workspace path (called when project is selected)
    pub fn set_workspace_path(&mut self, workspace_path: PathBuf) {
        self.workspace_path = Some(workspace_path);
    }
    
    
    /// Loads the system prompt from consolidated prompts file or uses default
    async fn load_system_prompt() -> String {
        // Try to load from consolidated prompts file first
        let config_path = PathBuf::from("prompts/latex-agent-systemprompt.md");
        
        match fs::read_to_string(&config_path).await {
            Ok(content) => {
                tracing::info!("Loaded system prompt from prompts/latex-agent-systemprompt.md");
                content
            }
            Err(_) => {
                tracing::warn!("Could not load system prompt from {:?}, using default", config_path);
                "You are a helpful LaTeX document assistant.".to_string()
            }
        }
    }
    
    /// Processes a chat request with background processing and real-time events
    pub async fn process_chat(&self, request: ChatRequest) -> AgentResult<ChatResponse> {
        // Validate workspace is available
        let workspace_path = self.workspace_path.as_ref()
            .ok_or_else(|| AgentError::ConfigError("No project workspace selected".to_string()))?;
        
        let job_id = Uuid::new_v4().to_string();
        
        // Emit job queued event
        self.event_broadcaster
            .broadcast(&request.session_id, AgentEvent::JobQueued { 
                job_id: job_id.clone(), 
                session_id: request.session_id.clone(),
                metadata: Self::create_metadata(&job_id),
            })
            .await;
        
        // Clone values for background processing
        let session_id = request.session_id.clone();
        let message = request.message.clone();
        let model = request.model.clone();
        let job_id_clone = job_id.clone();
        
        // Clone Arc references for background task
        let session_manager = self.session_manager.clone();
        let event_broadcaster = self.event_broadcaster.clone();
        let tool_registry = self.tool_registry.clone();
        let http_client = self.http_client.clone();
        let config = self.config.clone();
        let workspace_path = workspace_path.clone();
        let app_handle = self.app_handle.clone();
        
        // Process in background
        tokio::spawn(async move {
            tracing::info!("Starting background processing for job {} in session {}", job_id_clone, session_id);
            
            match Self::process_chat_background(
                session_id.clone(),
                message,
                model,
                session_manager,
                event_broadcaster.clone(),
                tool_registry,
                http_client,
                config,
                workspace_path,
                app_handle,
            ).await {
                Ok(response) => {
                    tracing::info!("Job {} completed successfully for session {} with response: {}", 
                                 job_id_clone, session_id, response);
                    
                    // Emit completion event
                    event_broadcaster
                        .broadcast(&session_id, AgentEvent::JobComplete { 
                            response,
                            metadata: EventMetadata::new(job_id_clone.clone()),
                        })
                        .await;
                    
                    tracing::info!("JobComplete event emitted for job {}", job_id_clone);
                }
                Err(e) => {
                    tracing::error!("Job {} failed for session {}: {}", 
                                  job_id_clone, session_id, e);
                    
                    // Emit error event
                    event_broadcaster
                        .broadcast(&session_id, AgentEvent::Error {
                            message: format!("Job failed: {}", e),
                            metadata: EventMetadata::new(job_id_clone.clone()),
                        })
                        .await;
                }
            }
        });
        
        Ok(ChatResponse {
            session_id: request.session_id,
            job_id,
        })
    }
    
    /// Background chat processing with full LLM integration and tool calling
    async fn process_chat_background(
        session_id: String,
        message: String,
        model: String,
        session_manager: Arc<SessionManager>,
        event_broadcaster: Arc<EventBroadcaster>,
        tool_registry: ToolRegistry,
        http_client: Client,
        config: AgentConfig,
        workspace_path: PathBuf,
        app_handle: tauri::AppHandle,
    ) -> AgentResult<String> {
        tracing::info!("Processing chat background for session {}", session_id);
        
        // Get or create session
        let _session = session_manager
            .get_or_create_session(session_id.clone(), None)
            .await;
        
        tracing::debug!("Session created/retrieved for {}", session_id);
        
        // Get session history
        let mut messages = session_manager
            .get_history(&session_id)
            .await?;
        
        tracing::debug!("Retrieved {} historical messages for session {}", messages.len(), session_id);
        
        // Add system prompt if this is the first message
        if messages.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(config.system_prompt.clone()),
                ..Default::default()
            });
        }
        
        // Add user message
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: Some(message),
            ..Default::default()
        };
        
        messages.push(user_message.clone());
        session_manager
            .add_message(&session_id, user_message)
            .await?;
        
        tracing::debug!("Starting tool processing for session {}", session_id);
        
        // Process with tool calling loop
        let response = Self::process_with_tools(
            messages,
            model.clone(),
            &session_id,
            session_manager.clone(),
            event_broadcaster,
            tool_registry,
            http_client,
            config,
            workspace_path,
            app_handle,
        ).await?;
        
        tracing::debug!("Tool processing completed for session {}", session_id);
        
        // Add assistant response to session
        session_manager
            .add_message(&session_id, response.clone())
            .await?;
        
        // Return content directly without additional parsing
        let final_content = response.content.unwrap_or_default();
        Ok(final_content)
    }
    
    /// Processes messages with tool calling loop and parallel execution
    async fn process_with_tools(
        mut messages: Vec<ChatMessage>,
        model: String,
        session_id: &str,
        session_manager: Arc<SessionManager>,
        event_broadcaster: Arc<EventBroadcaster>,
        tool_registry: ToolRegistry,
        http_client: Client,
        config: AgentConfig,
        workspace_path: PathBuf,
        app_handle: tauri::AppHandle,
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
            event_broadcaster
                .broadcast(session_id, AgentEvent::LLMCallStart { 
                    model: model.clone(),
                    metadata: EventMetadata::new(format!("llm-{}", iteration_count)),
                })
                .await;
            
            let response = Self::call_litellm(
                &messages,
                &model,
                &tool_registry,
                &http_client,
                &config,
                &event_broadcaster,
                session_id,
            ).await?;
            
            // Check for tool calls
            if let Some(tool_calls) = response.tool_calls.clone() {
                if tool_calls.is_empty() {
                    // No tool calls - return response directly without additional call
                    event_broadcaster
                        .broadcast(session_id, AgentEvent::LLMCallComplete {
                            metadata: EventMetadata::new(format!("llm-complete-{}", iteration_count)),
                        })
                        .await;
                    return Ok(response);
                }
                
                // Add assistant message with tool calls to history FIRST
                messages.push(response);
                
                // Execute tools (in parallel if multiple)
                if tool_calls.len() > 1 {
                    // Emit ToolCallRequested for each tool with UI metadata before execution
                    for tool_call in &tool_calls {
                        let (display_name, progressive_form) = tool_registry
                            .get_tool_metadata(&tool_call.function.name)
                            .unwrap_or((tool_call.function.name.clone(), format!("Running {}", tool_call.function.name)));
                        event_broadcaster
                            .broadcast(session_id, AgentEvent::ToolCallRequested {
                                tool: tool_call.function.name.clone(),
                                args: serde_json::from_str(&tool_call.function.arguments).unwrap_or(serde_json::Value::Null),
                                display_name: Some(display_name),
                                progressive_form: Some(progressive_form),
                                metadata: EventMetadata::new(format!("tool-req-{}", tool_call.id)),
                            })
                            .await;
                    }
                    // Multiple tools - execute in parallel
                    let tool_futures: Vec<_> = tool_calls
                        .iter()
                        .map(|tool_call| {
                            Self::execute_tool_async(
                                tool_call.clone(),
                                &tool_registry,
                                &workspace_path,
                                &app_handle,
                                &event_broadcaster,
                                session_id,
                            )
                        })
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
                            }
                            Err(e) => {
                                let error_msg = format!("Tool execution failed: {}", e);
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(error_msg),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                            }
                        }
                    }
                } else {
                    // Single tool - execute normally
                    for tool_call in &tool_calls {
                        // Emit ToolCallRequested prior to execution for consistent UI
                        let (display_name, progressive_form) = tool_registry
                            .get_tool_metadata(&tool_call.function.name)
                            .unwrap_or((tool_call.function.name.clone(), format!("Running {}", tool_call.function.name)));
                        event_broadcaster
                            .broadcast(session_id, AgentEvent::ToolCallRequested {
                                tool: tool_call.function.name.clone(),
                                args: serde_json::from_str(&tool_call.function.arguments).unwrap_or(serde_json::Value::Null),
                                display_name: Some(display_name),
                                progressive_form: Some(progressive_form),
                                metadata: EventMetadata::new(format!("tool-req-{}", tool_call.id)),
                            })
                            .await;
                        match Self::execute_tool(
                            &tool_call.function,
                            &tool_registry,
                            &workspace_path,
                            &app_handle,
                            &event_broadcaster,
                            session_id,
                        ).await {
                            Ok(result) => {
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(result),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                            }
                            Err(e) => {
                                let error_msg = format!("Tool execution failed: {}", e);
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: Some(error_msg),
                                    tool_call_id: Some(tool_call.id.clone()),
                                    tool_calls: None,
                                });
                            }
                        }
                    }
                }
                
                // Save updated messages to session
                for msg in &messages[messages.len() - tool_calls.len() - 1..] {
                    session_manager.add_message(session_id, msg.clone()).await?;
                }
                
                // Continue loop for next LLM call with tool results
            } else {
                // No tool calls, return final response
                event_broadcaster
                    .broadcast(session_id, AgentEvent::LLMCallComplete {
                        metadata: EventMetadata::new(format!("llm-complete-final")),
                    })
                    .await;
                return Ok(response);
            }
        }
    }
    
    /// Executes a tool asynchronously (for parallel execution)
    async fn execute_tool_async(
        tool_call: ToolCall,
        tool_registry: &ToolRegistry,
        workspace_path: &PathBuf,
        app_handle: &tauri::AppHandle,
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<String> {
        // Get tool metadata
        let (display_name, progressive_form) = tool_registry
            .get_tool_metadata(&tool_call.function.name)
            .unwrap_or((tool_call.function.name.clone(), format!("Running {}", tool_call.function.name)));
        
        event_broadcaster
            .broadcast(session_id, AgentEvent::ToolExecuting {
                tool: tool_call.function.name.clone(),
                display_name: Some(display_name),
                progressive_form: Some(progressive_form),
                metadata: EventMetadata::new(format!("tool-exec-{}", tool_call.id)),
            })
            .await;
        
        let result = Self::execute_tool(
            &tool_call.function,
            tool_registry,
            workspace_path,
            app_handle,
            event_broadcaster,
            session_id,
        ).await;
        
        match &result {
            Ok(tool_result) => {
                event_broadcaster
                    .broadcast(session_id, AgentEvent::ToolCompleted {
                        tool: tool_call.function.name.clone(),
                        result: tool_result.clone(),
                        metadata: Self::create_file_metadata(
                            &format!("tool-{}", tool_call.id),
                            &tool_call.function.name,
                            &tool_result
                        ),
                    })
                    .await;
            }
            Err(_) => {
                // Error event will be handled by caller
            }
        }
        
        result
    }
    
    /// Executes a single tool
    async fn execute_tool(
        tool_function: &ToolFunction,
        tool_registry: &ToolRegistry,
        workspace_path: &PathBuf,
        app_handle: &tauri::AppHandle,
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<String> {
        // Get tool metadata
        let (display_name, progressive_form) = tool_registry
            .get_tool_metadata(&tool_function.name)
            .unwrap_or((tool_function.name.clone(), format!("Running {}", tool_function.name)));
        
        event_broadcaster
            .broadcast(session_id, AgentEvent::ToolExecuting {
                tool: tool_function.name.clone(),
                display_name: Some(display_name),
                progressive_form: Some(progressive_form),
                metadata: EventMetadata::new(format!("sync-tool-exec")),
            })
            .await;
        
        let args: serde_json::Value = serde_json::from_str(&tool_function.arguments)
            .map_err(|e| AgentError::InvalidToolArguments {
                tool: tool_function.name.clone(),
                error: format!("Invalid JSON arguments: {}", e),
            })?;
        
        let result = tool_registry
            .execute(&tool_function.name, workspace_path, args, Some(app_handle))
            .await;
        
        match &result {
            Ok(tool_result) => {
                event_broadcaster
                    .broadcast(session_id, AgentEvent::ToolCompleted {
                        tool: tool_function.name.clone(),
                        result: tool_result.clone(),
                        metadata: Self::create_file_metadata(
                            &format!("tool-exec"),
                            &tool_function.name,
                            &tool_result
                        ),
                    })
                    .await;
            }
            Err(_) => {
                // Error will be propagated up
            }
        }
        
        result
    }
    
    /// Calls LiteLLM API with real SSE streaming and tool call buffering
    async fn call_litellm(
        messages: &[ChatMessage],
        model: &str,
        tool_registry: &ToolRegistry,
        http_client: &Client,
        config: &AgentConfig,
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<ChatMessage> {
        tracing::debug!("Calling LiteLLM for session {} with model {}", session_id, model);
        
        // Build streaming request with ALL tools included
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": tool_registry.get_definitions(),
            "tool_choice": "auto",
            "stream": true
        });
        
        tracing::debug!("Making streaming HTTP request to LiteLLM: {}/v1/chat/completions", config.litellm_base_url);
        
        let response = http_client
            .post(format!("{}/v1/chat/completions", config.litellm_base_url))
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
        
        // Process streaming response
        let mut stream = response.bytes_stream();
        let mut context = StreamingContext::new();
        let mut buffer = Vec::new();
        
        while let Some(chunk) = stream.next().await {
            let chunk_bytes = chunk.map_err(AgentError::HttpError)?;
            buffer.extend_from_slice(&chunk_bytes);
            
            // Process complete lines from buffer
            let buffer_str = String::from_utf8_lossy(&buffer);
            let lines: Vec<&str> = buffer_str.lines().collect();
            
            // Keep incomplete line in buffer
            if !buffer_str.ends_with('\n') && lines.len() > 0 {
                let incomplete_line = lines[lines.len()-1].to_string();
                let complete_lines = &lines[..lines.len()-1];
                
                for line in complete_lines {
                    Self::process_sse_line(line, &mut context, event_broadcaster, session_id).await;
                }
                
                buffer = incomplete_line.as_bytes().to_vec();
            } else {
                // All lines are complete
                for line in lines {
                    Self::process_sse_line(line, &mut context, event_broadcaster, session_id).await;
                }
                buffer.clear();
            }
        }
        
        Ok(context.into_message())
    }
    
    /// Processes a single SSE line for streaming content and tool calls
    async fn process_sse_line(
        line: &str,
        context: &mut StreamingContext,
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) {
        if line.starts_with("data: ") {
            let data = &line[6..]; // Remove "data: " prefix
            
            if data == "[DONE]" {
                return; // End of stream
            }
            
            // Try to parse as streaming chunk
            if let Ok(Some(chunk)) = context.process_chunk(data) {
                // Emit immediate streaming content
                if let Some(delta) = chunk.choices.first().and_then(|c| c.delta.as_ref()) {
                    if let Some(content) = &delta.content {
                        if !content.is_empty() {
                            event_broadcaster
                                .broadcast(session_id, AgentEvent::StreamChunk {
                                    content: content.clone(),
                                    metadata: EventMetadata::new("stream-chunk".to_string()),
                                })
                                .await;
                        }
                    }
                    
                    // Handle tool call events
                    if let Some(tool_calls) = &delta.tool_calls {
                        for tool_call_delta in tool_calls {
                            // Emit ToolCallStart for new tool calls
                            if let Some(id) = &tool_call_delta.id {
                                if let Some(name) = &tool_call_delta.function.name {
                                    event_broadcaster
                                        .broadcast(session_id, AgentEvent::ToolCallStart {
                                            tool_id: id.clone(),
                                            tool_name: name.clone(),
                                            metadata: EventMetadata::new(format!("tool-start-{}", id)),
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check if any tool calls are now complete and emit ToolCallReady events
        for tool_call in &context.tool_calls {
            event_broadcaster
                .broadcast(session_id, AgentEvent::ToolCallReady {
                    tool_call: tool_call.clone(),
                    metadata: EventMetadata::new(format!("tool-ready-{}", tool_call.id)),
                })
                .await;
        }
    }
    
    /// Subscribe to events for a session
    pub fn subscribe_to_session_events(&self, session_id: String) {
        self.event_broadcaster.subscribe(session_id);
    }
    
    /// Unsubscribe from events for a session
    pub fn unsubscribe_from_session_events(&self, session_id: &str) {
        self.event_broadcaster.unsubscribe(session_id);
    }
    
    /// Get session information
    pub async fn get_session_info(&self, session_id: &str) -> Option<crate::models::agent::SessionInfo> {
        self.session_manager.get_session_info(session_id).await
    }
    
    /// List all active sessions
    pub async fn list_active_sessions(&self) -> Vec<crate::models::agent::SessionInfo> {
        self.session_manager.list_sessions().await
    }
    
    /// Clear a specific session
    pub async fn clear_session(&self, session_id: &str) -> bool {
        // Also unsubscribe from events when clearing session
        self.event_broadcaster.unsubscribe(session_id);
        self.session_manager.remove_session(session_id).await
    }
    
    /// Get available tool definitions
    pub fn get_available_tools(&self) -> Vec<crate::models::agent::ToolDefinition> {
        self.tool_registry.get_definitions()
    }
}