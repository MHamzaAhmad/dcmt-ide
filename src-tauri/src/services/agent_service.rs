use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use reqwest::Client;
use uuid::Uuid;

use crate::models::agent::{
    AgentConfig, AgentEvent, AgentResult, AgentError,
    ChatMessage, ChatRequest, ChatResponse, LiteLLMRequest, LiteLLMResponse,
    ResponseFormat, ToolCall, ToolFunction,
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
}

impl AgentService {
    /// Creates a new agent service
    pub async fn new(
        event_broadcaster: Arc<EventBroadcaster>,
        litellm_base_url: String,
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
        })
    }
    
    /// Sets the workspace path (called when project is selected)
    pub fn set_workspace_path(&mut self, workspace_path: PathBuf) {
        self.workspace_path = Some(workspace_path);
    }
    
    
    /// Loads the system prompt from embedded config or uses default
    async fn load_system_prompt() -> String {
        // For desktop app, we can embed the system prompt or load from a config file
        // For now, using a default LaTeX-focused prompt
        r#"You are a helpful LaTeX document assistant. You have access to file operations to help users create, edit, and manage their LaTeX documents and projects.

Key responsibilities:
- Help create and edit LaTeX documents following best practices
- Assist with document structure, formatting, and organization  
- Provide clear explanations of LaTeX concepts and commands
- Use the available file tools to read, write, and modify files in the workspace
- Maintain clean, well-organized project structure

Available file operations:
- read_file: Examine existing files
- write_file: Create new files or overwrite existing ones
- update_file: Make targeted edits using find-and-replace
- list_files: Explore directory structure
- create_directory: Organize files into folders
- delete_file: Remove unnecessary files (use with caution)

Always explain your actions and provide educational context about LaTeX when appropriate."#.to_string()
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
                session_id: request.session_id.clone() 
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
        
        // Process in background
        tokio::spawn(async move {
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
            ).await {
                Ok(response) => {
                    // Emit completion event
                    event_broadcaster
                        .broadcast(&session_id, AgentEvent::JobComplete { 
                            response 
                        })
                        .await;
                }
                Err(e) => {
                    tracing::error!("Job {} failed for session {}: {}", 
                                  job_id_clone, session_id, e);
                    
                    // Emit error event
                    event_broadcaster
                        .broadcast(&session_id, AgentEvent::Error {
                            message: format!("Job failed: {}", e),
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
    ) -> AgentResult<String> {
        // Get or create session
        let _session = session_manager
            .get_or_create_session(session_id.clone(), None)
            .await;
        
        // Get session history
        let mut messages = session_manager
            .get_history(&session_id)
            .await?;
        
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
        
        // Process with tool calling loop
        let response = Self::process_with_tools(
            messages,
            model,
            &session_id,
            session_manager.clone(),
            event_broadcaster,
            tool_registry,
            http_client,
            config,
            workspace_path,
        ).await?;
        
        // Add assistant response to session
        session_manager
            .add_message(&session_id, response.clone())
            .await?;
        
        Ok(response.content.unwrap_or_default())
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
                    model: model.clone() 
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
                    // No tool calls, return response
                    event_broadcaster
                        .broadcast(session_id, AgentEvent::LLMCallComplete)
                        .await;
                    return Ok(response);
                }
                
                // Execute tools (in parallel if multiple)
                if tool_calls.len() > 1 {
                    // Multiple tools - execute in parallel
                    let tool_futures: Vec<_> = tool_calls
                        .iter()
                        .map(|tool_call| {
                            Self::execute_tool_async(
                                tool_call.clone(),
                                &tool_registry,
                                &workspace_path,
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
                        match Self::execute_tool(
                            &tool_call.function,
                            &tool_registry,
                            &workspace_path,
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
                
                // Add assistant message with tool calls to history
                messages.push(response);
                
                // Save updated messages to session
                for msg in &messages[messages.len() - tool_calls.len() - 1..] {
                    session_manager.add_message(session_id, msg.clone()).await?;
                }
                
                // Continue loop for next LLM call with tool results
            } else {
                // No tool calls, return final response
                event_broadcaster
                    .broadcast(session_id, AgentEvent::LLMCallComplete)
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
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<String> {
        event_broadcaster
            .broadcast(session_id, AgentEvent::ToolExecuting {
                tool: tool_call.function.name.clone(),
            })
            .await;
        
        let result = Self::execute_tool(
            &tool_call.function,
            tool_registry,
            workspace_path,
            event_broadcaster,
            session_id,
        ).await;
        
        match &result {
            Ok(tool_result) => {
                event_broadcaster
                    .broadcast(session_id, AgentEvent::ToolCompleted {
                        tool: tool_call.function.name.clone(),
                        result: tool_result.clone(),
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
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<String> {
        event_broadcaster
            .broadcast(session_id, AgentEvent::ToolExecuting {
                tool: tool_function.name.clone(),
            })
            .await;
        
        let args: serde_json::Value = serde_json::from_str(&tool_function.arguments)
            .map_err(|e| AgentError::InvalidToolArguments {
                tool: tool_function.name.clone(),
                error: format!("Invalid JSON arguments: {}", e),
            })?;
        
        let result = tool_registry
            .execute(&tool_function.name, workspace_path, args)
            .await;
        
        match &result {
            Ok(tool_result) => {
                event_broadcaster
                    .broadcast(session_id, AgentEvent::ToolCompleted {
                        tool: tool_function.name.clone(),
                        result: tool_result.clone(),
                    })
                    .await;
            }
            Err(_) => {
                // Error will be propagated up
            }
        }
        
        result
    }
    
    /// Calls LiteLLM API with all tools included and handles streaming
    async fn call_litellm(
        messages: &[ChatMessage],
        model: &str,
        tool_registry: &ToolRegistry,
        http_client: &Client,
        config: &AgentConfig,
        event_broadcaster: &Arc<EventBroadcaster>,
        session_id: &str,
    ) -> AgentResult<ChatMessage> {
        // Build request with ALL tools included
        let request = LiteLLMRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            tools: tool_registry.get_definitions(),
            tool_choice: "auto".to_string(),
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };
        
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
        
        // Parse structured JSON response
        let litellm_response: LiteLLMResponse = response.json().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        // Extract the first choice
        let choice = litellm_response.choices
            .into_iter()
            .next()
            .ok_or_else(|| AgentError::LiteLLMError {
                message: "No choices in LiteLLM response".to_string(),
            })?;
        
        // Emit streaming event for response content
        if let Some(content) = &choice.message.content {
            event_broadcaster
                .broadcast(session_id, AgentEvent::LLMStreaming {
                    content: content.clone(),
                })
                .await;
        }
        
        Ok(choice.message)
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