use anyhow::Result;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use std::time::Duration;

use crate::model::agent::{
    ChatMessage as AgentChatMessage,
    ToolDefinition as AgentToolDefinition,
    ResponseFormat as AgentResponseFormat,
};

/// LLM Model information from the API
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

/// LLM Models API response
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMModelsResponse {
    pub data: Vec<LLMModel>,
    pub object: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {message}")]
    Api { message: String },

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

/// Repository for LLM API operations
pub struct LLMRepository {
    client: Client,
    base_url: String,
}

impl LLMRepository {
    /// Creates a new LLM repository instance
    pub fn new(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            base_url,
        })
    }

    /// Fetches available models from LLM service (LiteLLM)
    pub async fn list_models(&self) -> Result<LLMModelsResponse, LLMError> {
        debug!("Fetching models from LLM service");

        let url = format!("{}/v1/models", self.base_url);

        let response = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }

        let models_response: LLMModelsResponse = response.json().await
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;

        debug!("Successfully fetched {} models", models_response.data.len());
        Ok(models_response)
    }

    /// Request payload for chat completions
    #[allow(dead_code)]
    pub fn chat_endpoint(&self) -> String { format!("{}/v1/chat/completions", self.base_url) }

    /// Typed chat completion request
    #[allow(dead_code)]
    pub fn new_chat_request(
        &self,
        model: impl Into<String>,
        messages: Vec<AgentChatMessage>,
    ) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.into(),
            messages,
            tools: None,
            tool_choice: None,
            stream: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
        }
    }

    /// Create a chat completion (non-streaming)
    pub async fn create_chat_completion(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse, LLMError> {
        debug!("Creating chat completion (non-streaming) with model: {}", req.model);

        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(req)
            .send()
            .await?;

        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }

        let parsed: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| LLMError::InvalidResponse(e.to_string()))?;
        Ok(parsed)
    }

    /// Create a streaming chat completion and return the raw HTTP response for SSE consumption
    pub async fn create_chat_completion_stream(&self, mut req: ChatCompletionRequest) -> Result<Response, LLMError> {
        debug!("Creating streaming chat completion with model: {}", req.model);

        // Ensure streaming flag is set
        req.stream = Some(true);

        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }

        Ok(response)
    }

    /// Handles error responses from the LLM API
    async fn handle_error_response<T>(&self, response: reqwest::Response) -> Result<T, LLMError> {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        error!("LLM API error {}: {}", status, error_text);

        Err(LLMError::Api {
            message: format!("HTTP {}: {}", status, error_text)
        })
    }
}

/// Chat completion structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<AgentChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AgentToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<AgentResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: AgentChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}