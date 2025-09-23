use anyhow::Result;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::time::Duration;

use crate::models::agent::{ChatMessage, ToolDefinition, LiteLLMRequest, LiteLLMResponse};

#[derive(Clone)]
pub struct LLMRepository {
    client: Client,
    base_url: String,
}

impl LLMRepository {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            // Default for non-streaming requests
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, base_url }
    }

    fn models_endpoint(&self) -> String {
        format!("{}/v1/models", self.base_url)
    }

    fn chat_endpoint(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    /// List available models (LiteLLM-compatible response)
    pub async fn list_models(&self) -> Result<LiteLLMModelsResponse> {
        let res = self.client.get(self.models_endpoint()).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        let parsed = res.json::<LiteLLMModelsResponse>().await?;
        Ok(parsed)
    }

    /// Create chat completion (JSON, non-streaming) using the standard LiteLLMRequest/Response types
    pub async fn create_chat_completion(&self, req: &LiteLLMRequest) -> Result<LiteLLMResponse> {
        self.create_chat_completion_with_body::<_, LiteLLMResponse>(req).await
    }

    /// Generic helper to post any JSON body and parse a typed response
    pub async fn create_chat_completion_with_body<T: Serialize, R: DeserializeOwned>(&self, body: &T) -> Result<R> {
        let res = self.client.post(self.chat_endpoint()).json(body).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        let parsed = res.json::<R>().await?;
        Ok(parsed)
    }

    /// Streaming chat completion: returns the raw HTTP response to consume as SSE/bytes stream
    pub async fn create_chat_completion_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<Response> {
        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            // Allow the model to decide when to call tools and when to stop
            "tool_choice": "auto",
            "stream": true
        });

        let res = self.client
            .post(self.chat_endpoint())
            .json(&payload)
            // Streaming may take longer; set a generous timeout
            .timeout(Duration::from_secs(600))
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        Ok(res)
    }
}

// LiteLLM Models types (compatible with frontend LiteLLMModelsResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMModelsResponse {
    pub data: Vec<LiteLLMModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
    #[serde(flatten)]
    pub extra: Option<serde_json::Value>,
}
