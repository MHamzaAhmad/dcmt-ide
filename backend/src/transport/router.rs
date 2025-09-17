use crate::{config::Config, svc::{AgentService, FileService, LaTeXService, GitService, LLMService}, repo::{AgentRepo, tavily::TavilyRepository}};
use crate::transport::middleware::{cors::create_cors_layer, logging::create_trace_layer, create_clerk_auth_layer};
use crate::transport::routes::{agent_router, files_router, latex_router, sse_router, websocket_router, git_router, llm_router};
use crate::transport::routes::websocket::WebSocketServices;
use anyhow::Result;
use axum::Router;
use std::sync::Arc;

pub async fn create_router(config: Config) -> Result<Router> {
    // Create services with config
    let file_service = Arc::new(FileService::new(config.workspace_path.clone())?);
    let latex_service = Arc::new(LaTeXService::new(config.workspace_path.clone()));
    
    // Connect file watcher to LaTeX service for automatic compilation
    {
        let latex_service_clone = latex_service.clone();
        let mut file_receiver = file_service.subscribe_to_events();
        tokio::spawn(async move {
            while let Ok(file_event) = file_receiver.recv().await {
                // Check if this is a LaTeX-related file modification
                if file_event.event_type == crate::model::FileEventType::Modified {
                    latex_service_clone.handle_file_change(&file_event).await;
                }
            }
        });
    }
    let git_service = Arc::new(GitService::new(
        config.workspace_path.clone(),
        config.agent.litellm_base_url.clone(),
    )?);

    // Create Tavily repository with API key
    let tavily_repo = match config.tavily_api_key {
        Some(api_key) => Arc::new(TavilyRepository::new(api_key)?),
        None => {
            tracing::warn!("TAVILY_API_KEY not configured - web search and extraction will be unavailable");
            return Err(anyhow::anyhow!("TAVILY_API_KEY is required for web search functionality"));
        }
    };

    // Create agent repository and service
    let agent_repo = Arc::new(AgentRepo::new(
        config.workspace_path.clone(),
        config.agent.litellm_base_url.clone(),
        file_service.clone(),
        latex_service.clone(),
        tavily_repo,
    ).await?);
    let agent_service = Arc::new(AgentService::new(agent_repo));

    // Create LLM service
    let llm_service = Arc::new(LLMService::new(config.agent.litellm_base_url.clone())?);

    // Create separate routers for different services
    let files_routes = files_router().with_state(file_service.clone());
    let latex_routes = latex_router().with_state(latex_service.clone());
    let git_routes = git_router().with_state(git_service);
    let agent_routes = agent_router().with_state(agent_service.clone());
    let sse_routes = sse_router().with_state(agent_service.clone());
    let llm_routes = llm_router().with_state(llm_service.clone());
    
    // Create API routes by nesting sub-routers with Clerk authentication
    let api_routes = Router::new()
        .nest("/files", files_routes)
        .nest("/latex", latex_routes)
        .nest("/git", git_routes)
        .nest("/agent", agent_routes)
        .nest("/llm", llm_routes)
        .layer(create_clerk_auth_layer(config.clerk_secret_key.clone()));

    // Create combined WebSocket services state
    let websocket_services = WebSocketServices {
        file_service: file_service.clone(),
        agent_service: agent_service,
        latex_service: latex_service.clone(),
    };
    
    // Main application router
    let app = Router::new()
        .nest("/api", api_routes)
        .nest("/sse", sse_routes)
        .nest("/ws", websocket_router().with_state(websocket_services))
        .layer(create_cors_layer())
        .layer(create_trace_layer());

    Ok(app)
}