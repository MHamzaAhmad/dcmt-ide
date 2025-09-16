use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use std::time::Duration;

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