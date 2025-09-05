use crate::{client::*, types::*};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

/// Desktop LLM client using reqwest with full async support
#[derive(Debug)]
pub struct DesktopLlmClient {
    client: reqwest::Client,
    config: LlmClientConfig,
}

impl DesktopLlmClient {
    pub fn new(config: LlmClientConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        
        if !config.auth_token.is_empty() {
            if let Ok(auth_header) = format!("Bearer {}", config.auth_token).parse() {
                headers.insert(reqwest::header::AUTHORIZATION, auth_header);
            }
        }
        
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self { client, config }
    }
}

impl LlmClient for DesktopLlmClient {
    async fn list_models(&self, provider: Option<Provider>) -> Result<ModelsResponse, LlmError> {
        let provider_param = provider
            .map(|p| format!("?provider={}", p.as_str()))
            .unwrap_or_default();
        
        let url = format!("{}/llm/models{}", self.config.base_url, provider_param);
        
        tracing::debug!("Fetching models from: {}", url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(LlmError::ApiError {
                status: response.status().as_u16(),
                message: format!("HTTP error: {}", response.status()),
            });
        }
        
        let models: ModelsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::JsonError(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("JSON parse error: {}", e)
            ))))?;
        
        tracing::debug!("Received {} models", models.data.len());
        
        Ok(models)
    }
    
    async fn chat_completions_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, LlmError>>>>, LlmError> {
        let url = format!("{}/llm/chat/completions", self.config.base_url);
        
        tracing::debug!("Starting chat completion stream to: {}", url);
        
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(LlmError::ApiError {
                status: response.status().as_u16(),
                message: format!("HTTP error: {}", response.status()),
            });
        }
        
        // Convert response to proper SSE stream
        use std::sync::Arc;
        use std::sync::Mutex;
        
        let byte_stream = response.bytes_stream();
        let buffer = Arc::new(Mutex::new(String::new()));
        
        let sse_stream = byte_stream.filter_map({
            let buffer = Arc::clone(&buffer);
            move |bytes_result| {
                let buffer = Arc::clone(&buffer);
                async move {
                    let bytes = match bytes_result {
                        Ok(bytes) => bytes,
                        Err(e) => return Some(Err(LlmError::StreamError(format!("Stream error: {}", e)))),
                    };
                    
                    // Convert bytes to string and add to buffer
                    let chunk_text = match String::from_utf8(bytes.to_vec()) {
                        Ok(text) => text,
                        Err(e) => return Some(Err(LlmError::StreamError(format!("UTF-8 decode error: {}", e)))),
                    };
                    
                    // Lock buffer and process complete messages
                    let mut buffer_guard = buffer.lock().unwrap();
                    buffer_guard.push_str(&chunk_text);
                    
                    // Look for complete SSE messages (ending with \n\n)
                    if let Some(end_pos) = buffer_guard.find("\n\n") {
                        let complete_message = buffer_guard[..end_pos].to_string();
                        *buffer_guard = buffer_guard[end_pos + 2..].to_string();
                        drop(buffer_guard);
                        
                        // Parse the complete SSE message
                        match parse_sse_message(&complete_message) {
                            Ok(Some(chunk)) => Some(Ok(chunk)),
                            Ok(None) => None, // Skip empty or control messages, continue reading
                            Err(e) => Some(Err(e)),
                        }
                    } else {
                        None // Incomplete message, continue reading
                    }
                }
            }
        });
        
        Ok(Box::pin(sse_stream))
    }
}

/// Parse a complete SSE message
fn parse_sse_message(message: &str) -> Result<Option<ChatCompletionChunk>, LlmError> {
    // Split message into lines
    let lines: Vec<&str> = message.lines().collect();
    
    // Find data lines
    for line in lines {
        if let Some(data) = line.strip_prefix("data: ") {
            // Skip [DONE] markers
            if data.trim() == "[DONE]" {
                return Ok(None);
            }
            
            // Parse JSON data
            if !data.trim().is_empty() {
                match serde_json::from_str::<ChatCompletionChunk>(data) {
                    Ok(chunk) => return Ok(Some(chunk)),
                    Err(e) => {
                        tracing::warn!("Failed to parse SSE data: {} - Error: {}", data, e);
                        return Err(LlmError::StreamError(format!("JSON parse error: {}", e)));
                    }
                }
            }
        }
    }
    
    // No data found, continue
    Ok(None)
}