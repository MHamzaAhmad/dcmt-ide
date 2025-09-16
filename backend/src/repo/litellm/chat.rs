use super::types::*;
use super::uds_http::{make_uds_request, create_uds_uri};
use anyhow::Result;
use hyper::Body;
use hyper::Request;
use hyper::http::header;
use hyper::Method;
use serde_json;
use tracing::{debug, error};

pub struct ChatClient {
    socket_path: String,
}

impl ChatClient {
    pub fn new(socket_path: String) -> Result<Self> {
        Ok(Self { socket_path })
    }

    /// Create chat completion via UDS
    pub async fn create_chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        debug!("Creating chat completion with model: {}", request.model);

        // Create request body
        let request_body = serde_json::to_vec(&request)
            .map_err(|e| {
                error!("Failed to serialize chat request: {}", e);
                LiteLLMError::Serialization(e.to_string())
            })?;

        // Build HTTP request
        let uri = create_uds_uri("/v1/chat/completions")?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::HOST, "litellm")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, "dcmt-litellm-client/1.0")
            .header(header::CONTENT_LENGTH, request_body.len())
            .body(Body::from(request_body))?;

        // Send request via UDS
        let response = make_uds_request(&self.socket_path, request).await?;

        // Check response status
        if response.status() != hyper::StatusCode::OK {
            let status = response.status();
            let body = hyper::body::to_bytes(response.into_body()).await?;
            let error_text = String::from_utf8_lossy(&body);
            error!("Chat completion request failed: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("Chat completion failed: {}", error_text));
        }

        // Parse response body
        let body_bytes = hyper::body::to_bytes(response.into_body()).await?;
        let chat_response: ChatCompletionResponse = serde_json::from_slice(&body_bytes)
            .map_err(|e| {
                error!("Failed to parse chat completion response: {}", e);
                LiteLLMError::InvalidResponse(e.to_string())
            })?;

        debug!("Chat completion successful, used {} tokens",
               chat_response.usage.as_ref().map_or(0, |u| u.total_tokens));
        Ok(chat_response)
    }

    /// Create streaming chat completion via UDS
    pub async fn create_streaming_chat_completion(&self, request: ChatCompletionRequest) -> Result<hyper::Response<Body>> {
        debug!("Creating streaming chat completion with model: {}", request.model);

        // Create streaming request
        let mut streaming_request = request;
        streaming_request.stream = Some(true);

        // Create request body
        let request_body = serde_json::to_vec(&streaming_request)
            .map_err(|e| {
                error!("Failed to serialize streaming chat request: {}", e);
                LiteLLMError::Serialization(e.to_string())
            })?;

        // Build HTTP request with streaming headers
        let uri = create_uds_uri("/v1/chat/completions")?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::HOST, "litellm")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::USER_AGENT, "dcmt-litellm-client/1.0")
            .header(header::CONTENT_LENGTH, request_body.len())
            .body(Body::from(request_body))?;

        // Send request via UDS
        let response = make_uds_request(&self.socket_path, request).await?;

        if response.status() != hyper::StatusCode::OK {
            let status = response.status();
            let body = hyper::body::to_bytes(response.into_body()).await?;
            let error_text = String::from_utf8_lossy(&body);
            error!("Streaming chat completion request failed: {} - {}", status, error_text);
            return Err(anyhow::anyhow!("Streaming chat completion failed: {}", error_text));
        }

        Ok(response)
    }
}