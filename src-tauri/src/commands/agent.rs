use tauri::{command, State};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::agent::{ChatRequest, ChatResponse, SessionInfo, ToolDefinition};
use crate::repo::llm::LLMRepository;
use crate::repo::llm::LiteLLMModelsResponse;
use crate::services::agent_service::AgentService;

/// Chat with the agent - processes in background and returns job_id immediately
#[command]
pub async fn chat_with_agent(
    session_id: String,
    message: String,
    model: String,
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<ChatResponse, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized. Please select a project first.")?;
    
    let request = ChatRequest {
        session_id,
        message,
        model,
    };
    
    service.process_chat(request)
        .await
        .map_err(|e| format!("Failed to process chat: {}", e))
}

/// Subscribe to agent events for a specific session
#[command]
pub async fn subscribe_to_agent_events(
    session_id: String,
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<(), String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    service.subscribe_to_session_events(session_id);
    Ok(())
}

/// Unsubscribe from agent events for a specific session
#[command]
pub async fn unsubscribe_from_agent_events(
    session_id: String,
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<(), String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    service.unsubscribe_from_session_events(&session_id);
    Ok(())
}

/// Get information about a specific session
#[command]
pub async fn get_agent_session_info(
    session_id: String,
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<Option<SessionInfo>, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    Ok(service.get_session_info(&session_id).await)
}

/// List all active agent sessions
#[command]
pub async fn list_agent_sessions(
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<Vec<SessionInfo>, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    Ok(service.list_active_sessions().await)
}

/// Clear/remove a specific agent session
#[command]
pub async fn clear_agent_session(
    session_id: String,
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<bool, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    Ok(service.clear_session(&session_id).await)
}

/// Get list of available agent tools
#[command]
pub async fn get_available_agent_tools(
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<Vec<ToolDefinition>, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;
    
    Ok(service.get_available_tools())
}

/// Check if agent service is available (project is selected)
#[command]
pub async fn is_agent_available(
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<bool, String> {
    let service_guard = agent_service.read().await;
    Ok(service_guard.is_some())
}

/// List available LLM models via LiteLLM (desktop backend route)
#[command]
pub async fn list_llm_models(
    agent_service: State<'_, Arc<RwLock<Option<AgentService>>>>,
) -> Result<LiteLLMModelsResponse, String> {
    let service_guard = agent_service.read().await;
    let service = service_guard.as_ref()
        .ok_or("Agent service not initialized")?;

    let repo = LLMRepository::new(service.litellm_base_url().to_string());
    repo.list_models().await.map_err(|e| format!("Failed to list models: {}", e))
}