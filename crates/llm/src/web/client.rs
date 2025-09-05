use crate::{client::*, types::*};
use futures::stream::Stream;
use std::pin::Pin;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, ReadableStreamDefaultReader};

/// Web-based LLM client using fetch API and web streams
pub struct WebLlmClient {
    config: LlmClientConfig,
}

impl WebLlmClient {
    pub fn new(config: LlmClientConfig) -> Self {
        Self { config }
    }
    
    /// Create a GET request to the API
    async fn get_request(&self, url: &str) -> Result<Response, LlmError> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        
        // Add headers
        let headers = web_sys::Headers::new().map_err(|e| {
            LlmError::NetworkError(format!("Failed to create headers: {:?}", e))
        })?;
        
        if !self.config.auth_token.is_empty() {
            headers.set("Authorization", &format!("Bearer {}", self.config.auth_token))
                .map_err(|e| LlmError::AuthError(format!("Failed to set auth header: {:?}", e)))?;
        }
        
        opts.set_headers(&headers);
        opts.set_mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| LlmError::NetworkError(format!("Failed to create request: {:?}", e)))?;
        
        let window = web_sys::window()
            .ok_or_else(|| LlmError::NetworkError("No window available".to_string()))?;
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| LlmError::NetworkError(format!("Fetch failed: {:?}", e)))?;
        
        let resp: Response = resp_value.dyn_into()
            .map_err(|e| LlmError::NetworkError(format!("Failed to cast response: {:?}", e)))?;
        
        if !resp.ok() {
            return Err(LlmError::ApiError {
                status: resp.status(),
                message: format!("HTTP error: {}", resp.status_text()),
            });
        }
        
        Ok(resp)
    }
    
    /// Create a POST request with JSON body
    async fn post_request(&self, url: &str, body: &str) -> Result<Response, LlmError> {
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_body(&JsValue::from_str(body));
        
        // Add headers
        let headers = web_sys::Headers::new().map_err(|e| {
            LlmError::NetworkError(format!("Failed to create headers: {:?}", e))
        })?;
        
        headers.set("Content-Type", "application/json")
            .map_err(|e| LlmError::NetworkError(format!("Failed to set content type: {:?}", e)))?;
        
        if !self.config.auth_token.is_empty() {
            headers.set("Authorization", &format!("Bearer {}", self.config.auth_token))
                .map_err(|e| LlmError::AuthError(format!("Failed to set auth header: {:?}", e)))?;
        }
        
        opts.set_headers(&headers);
        opts.set_mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| LlmError::NetworkError(format!("Failed to create request: {:?}", e)))?;
        
        let window = web_sys::window()
            .ok_or_else(|| LlmError::NetworkError("No window available".to_string()))?;
        
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| LlmError::NetworkError(format!("Fetch failed: {:?}", e)))?;
        
        let resp: Response = resp_value.dyn_into()
            .map_err(|e| LlmError::NetworkError(format!("Failed to cast response: {:?}", e)))?;
        
        if !resp.ok() {
            return Err(LlmError::ApiError {
                status: resp.status(),
                message: format!("HTTP error: {}", resp.status_text()),
            });
        }
        
        Ok(resp)
    }
}

impl LlmClient for WebLlmClient {
    async fn list_models(&self, provider: Option<Provider>) -> Result<ModelsResponse, LlmError> {
        let provider_param = provider
            .map(|p| format!("?provider={}", p.as_str()))
            .unwrap_or_default();
        
        let url = format!("{}/llm/models{}", self.config.base_url, provider_param);
        
        tracing::debug!("Fetching models from: {}", url);
        
        let response = self.get_request(&url).await?;
        
        let text_promise = response.text()
            .map_err(|e| LlmError::NetworkError(format!("Failed to get response text: {:?}", e)))?;
        
        let text_value = JsFuture::from(text_promise).await
            .map_err(|e| LlmError::NetworkError(format!("Failed to read response: {:?}", e)))?;
        
        let text = text_value.as_string()
            .ok_or_else(|| LlmError::NetworkError("Response is not a string".to_string()))?;
        
        let models: ModelsResponse = serde_json::from_str(&text)?;
        
        tracing::debug!("Received {} models", models.data.len());
        
        Ok(models)
    }
    
    async fn chat_completions_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, LlmError>>>>, LlmError> {
        let url = format!("{}/llm/chat/completions", self.config.base_url);
        let body = serde_json::to_string(&request)?;
        
        tracing::debug!("Starting chat completion stream to: {}", url);
        
        let response = self.post_request(&url, &body).await?;
        
        // Get the response body as a readable stream
        let body_stream = response.body()
            .ok_or_else(|| LlmError::StreamError("No response body".to_string()))?;
        
        let reader: ReadableStreamDefaultReader = body_stream.get_reader().dyn_into()
            .map_err(|_| LlmError::StreamError("Failed to get stream reader".to_string()))?;
        
        // Create a proper async stream that processes SSE chunks
        let stream = futures::stream::unfold(
            (reader, String::new()), 
            |(reader, mut buffer)| async move {
                loop {
                    // Read next chunk from stream
                    let read_promise = reader.read();
                    let read_result = match JsFuture::from(read_promise).await {
                        Ok(result) => result,
                        Err(e) => return Some((Err(LlmError::StreamError(format!("Stream read error: {:?}", e))), (reader, buffer))),
                    };
                    
                    // Check if stream is done
                    let done = js_sys::Reflect::get(&read_result, &JsValue::from_str("done"))
                        .unwrap_or(JsValue::TRUE);
                    
                    if done.is_truthy() {
                        return None; // Stream ended
                    }
                    
                    // Get chunk value
                    let value = js_sys::Reflect::get(&read_result, &JsValue::from_str("value"))
                        .unwrap_or(JsValue::UNDEFINED);
                    
                    if value.is_undefined() {
                        continue;
                    }
                    
                    // Convert to bytes
                    let uint8_array: js_sys::Uint8Array = match value.dyn_into() {
                        Ok(arr) => arr,
                        Err(_) => continue,
                    };
                    
                    let bytes = uint8_array.to_vec();
                    let chunk_text = match String::from_utf8(bytes) {
                        Ok(text) => text,
                        Err(e) => return Some((Err(LlmError::StreamError(format!("UTF-8 decode error: {}", e))), (reader, buffer))),
                    };
                    
                    // Add to buffer
                    buffer.push_str(&chunk_text);
                    
                    // Process complete SSE messages
                    if let Some(end_pos) = buffer.find("\n\n") {
                        let complete_message = buffer[..end_pos].to_string();
                        buffer = buffer[end_pos + 2..].to_string();
                        
                        // Parse SSE message
                        if let Some(data_line) = complete_message.lines().find(|line| line.starts_with("data: ")) {
                            let json_str = data_line.strip_prefix("data: ").unwrap_or(data_line);
                            
                            // Skip [DONE] markers
                            if json_str.trim() == "[DONE]" {
                                return None;
                            }
                            
                            match serde_json::from_str::<ChatCompletionChunk>(json_str) {
                                Ok(chunk) => return Some((Ok(chunk), (reader, buffer))),
                                Err(e) => {
                                    tracing::warn!("Failed to parse SSE chunk: {} - Error: {}", json_str, e);
                                    // Continue reading instead of returning error
                                    continue;
                                }
                            }
                        }
                    }
                    // Continue reading if no complete message yet
                }
            }
        );
        
        Ok(Box::pin(stream))
    }
}

