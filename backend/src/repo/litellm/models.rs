use super::types::*;
use super::uds_http::make_uds_request;
use anyhow::Result;
use hyper::Body;
use hyper::Request;
use hyper::http::header;
use hyper::Method;
use tracing::{debug, error};

pub struct ModelsClient {
    socket_path: String,
}

impl ModelsClient {
    pub fn new(socket_path: String) -> Result<Self> {
        Ok(Self { socket_path })
    }

    /// List available models via UDS
    pub async fn list_models(&self) -> Result<LiteLLMModelsResponse> {
        debug!("Fetching models from LiteLLM via UDS: {}", self.socket_path);

        // Build HTTP request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/v1/models")
            .header(header::HOST, "litellm")
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, "dcmt-litellm-client/1.0")
            .body(Body::empty())?;

        // Send request via UDS
        let response = make_uds_request(&self.socket_path, request).await?;

        if response.status() != hyper::StatusCode::OK {
            let status = response.status();
            let body = hyper::body::to_bytes(response.into_body()).await?;
            let error_text = String::from_utf8_lossy(&body);
            error!("Models request failed: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("Models request failed: {}", error_text));
        }

        // Parse response body
        let body_bytes = hyper::body::to_bytes(response.into_body()).await?;
        let models_response: LiteLLMModelsResponse = serde_json::from_slice(&body_bytes)
            .map_err(|e| {
                error!("Failed to parse models response: {}", e);
                LiteLLMError::InvalidResponse(e.to_string())
            })?;

        debug!("Successfully fetched {} models", models_response.data.len());
        Ok(models_response)
    }
}