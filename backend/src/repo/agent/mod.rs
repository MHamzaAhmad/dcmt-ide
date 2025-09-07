use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use reqwest::Client;
use tokio::fs;
use uuid::Uuid;

use crate::model::agent::{
    AgentConfig, AgentEvent, AgentResult, AgentError,
    ChatMessage, ChatRequest, LiteLLMRequest, LiteLLMResponse,
    ResponseFormat, ToolCall, ToolFunction,
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

impl AgentRepo {
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
                session_id: request.session_id.clone() 
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
        
        let response_content = response.content.unwrap_or_default();
        
        // Notify completion
        self.event_broadcaster
            .broadcast(&request.session_id, AgentEvent::JobComplete { 
                response: response_content.clone() 
            })
            .await;
        
        Ok(response_content)
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
                    model: model.clone() 
                })
                .await;
            
            let response = self.call_litellm(&messages, &model).await?;
            
            // Check for tool calls
            if let Some(tool_calls) = &response.tool_calls {
                if tool_calls.is_empty() {
                    // No tool calls, return response
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::LLMCallComplete)
                        .await;
                    return Ok(response);
                }
                
                if tool_calls.len() > 1 {
                    // Multiple tools - execute in parallel
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::ParallelToolsStart { 
                            count: tool_calls.len() 
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
                                    })
                                    .await;
                            }
                        }
                    }
                    
                    self.event_broadcaster
                        .broadcast(session_id, AgentEvent::ParallelToolsComplete { 
                            count: tool_calls.len() 
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
                                        result,
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
                                    })
                                    .await;
                            }
                        }
                    }
                }
                
                // Add assistant message with tool calls to history
                messages.push(response);
                
                // Continue loop for next LLM call with tool results
            } else {
                // No tool calls, return final response
                self.event_broadcaster
                    .broadcast(session_id, AgentEvent::LLMCallComplete)
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
    
    /// Calls LiteLLM API with all tools included
    async fn call_litellm(&self, messages: &[ChatMessage], model: &str) -> AgentResult<ChatMessage> {
        // Build request with ALL tools included
        let request = LiteLLMRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            tools: self.tool_registry.get_definitions(), // All tools always included
            tool_choice: "auto".to_string(),
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };
        
        let response = self.http_client
            .post(format!("{}/v1/chat/completions", self.config.litellm_base_url))
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
        
        Ok(choice.message)
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