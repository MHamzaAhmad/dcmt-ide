use anyhow::Result;
use std::env;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub mod types;
pub mod models;
pub mod chat;

pub use types::*;
pub use models::ModelsClient;
pub use chat::ChatClient;

/// Main LiteLLM repository for accessing LiteLLM functionality via UDS
#[derive(Clone)]
pub struct LiteLLMRepository {
    models_client: Arc<ModelsClient>,
    chat_client: Arc<ChatClient>,
    socket_path: String,
}

impl LiteLLMRepository {
    /// Creates a new LiteLLM repository instance using UDS
    pub fn new() -> Result<Self> {
        // Get UDS socket path from environment, default to standard location
        let socket_path = env::var("LITELLM_SOCKET_PATH")
            .unwrap_or_else(|_| "/app/litellm.sock".to_string());

        info!("Initializing LiteLLM repository with UDS: {}", socket_path);

        let models_client = Arc::new(ModelsClient::new(socket_path.clone())?);
        let chat_client = Arc::new(ChatClient::new(socket_path.clone())?);

        Ok(Self {
            models_client,
            chat_client,
            socket_path,
        })
    }

    /// Lists available models from LiteLLM
    pub async fn list_models(&self) -> Result<LiteLLMModelsResponse> {
        debug!("Listing available models");
        self.models_client.list_models().await
    }

    /// Creates a chat completion request
    pub async fn create_chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        debug!("Creating chat completion with model: {}", request.model);
        self.chat_client.create_chat_completion(request).await
    }

    /// Creates a streaming chat completion request
    pub async fn create_streaming_chat_completion(&self, request: ChatCompletionRequest) -> Result<std::os::unix::net::UnixStream> {
        debug!("Creating streaming chat completion with model: {}", request.model);
        self.chat_client.create_streaming_chat_completion(request).await
    }

    /// Converts LiteLLM models response to backend API format
    pub fn convert_models_response(&self, litellm_response: LiteLLMModelsResponse) -> ModelsResponse {
        let models: Vec<LLMModel> = litellm_response.data.into_iter().map(|model| {
            let name = model.id
                .replace("gpt-", "GPT-")
                .replace("claude-", "Claude ")
                .replace("gemini-", "Gemini ");

            LLMModel {
                id: model.id.clone(),
                name,
                description: format!("{} model", model.owned_by),
            }
        }).collect();

        ModelsResponse { models }
    }

    /// Converts backend chat request to LiteLLM format
    pub fn convert_chat_request(&self, request: ChatRequest) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: request.model,
            messages: request.messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: None, // Will be added when needed for tool calling
            tool_choice: None,
        }
    }

    /// Converts LiteLLM chat response to backend API format
    pub fn convert_chat_response(&self, litellm_response: ChatCompletionResponse) -> ChatResponse {
        let message = litellm_response.choices.first()
            .map(|choice| choice.message.clone())
            .unwrap_or_else(|| ChatMessage {
                role: "assistant".to_string(),
                content: "No response available".to_string(),
            });

        ChatResponse {
            id: litellm_response.id,
            model: litellm_response.model,
            message,
            usage: litellm_response.usage,
        }
    }

    /// Returns the socket path for monitoring purposes
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// Checks if the UDS socket is accessible
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Performing health check for LiteLLM UDS socket");

        match std::os::unix::net::UnixStream::connect(&self.socket_path) {
            Ok(_) => {
                info!("LiteLLM UDS socket health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("LiteLLM UDS socket health check failed: {}", e);
                Ok(false)
            }
        }
    }
}