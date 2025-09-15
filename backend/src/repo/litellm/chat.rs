use super::types::*;
use anyhow::Result;
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::time::Duration;
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

        // Connect to UDS socket
        let mut socket = self.connect_with_retry().await?;

        // Send HTTP request to LiteLLM
        let http_request = self.create_chat_request(request)?;
        self.send_request(&mut socket, &http_request).await?;

        // Read response
        let response = self.read_response(&mut socket).await?;

        // Parse as LiteLLM chat completion response
        let chat_response: ChatCompletionResponse = serde_json::from_str(&response)
            .map_err(|e| {
                error!("Failed to parse chat completion response: {}", e);
                LiteLLMError::InvalidResponse(e.to_string())
            })?;

        debug!("Chat completion successful, used {} tokens",
               chat_response.usage.as_ref().map_or(0, |u| u.total_tokens));
        Ok(chat_response)
    }

    /// Create streaming chat completion via UDS
    pub async fn create_streaming_chat_completion(&self, request: ChatCompletionRequest) -> Result<UnixStream> {
        debug!("Creating streaming chat completion with model: {}", request.model);

        // Connect to UDS socket
        let mut socket = self.connect_with_retry().await?;

        // Send HTTP request to LiteLLM
        let http_request = self.create_streaming_chat_request(request)?;
        self.send_request(&mut socket, &http_request).await?;

        Ok(socket)
    }

    async fn connect_with_retry(&self) -> Result<UnixStream> {
        let mut attempts = 0;
        let max_attempts = 5;

        loop {
            match UnixStream::connect(&self.socket_path) {
                Ok(socket) => {
                    socket.set_nonblocking(true)?;
                    socket.set_read_timeout(Some(Duration::from_secs(30)))?;
                    socket.set_write_timeout(Some(Duration::from_secs(30)))?;
                    return Ok(socket);
                }
                Err(_e) if attempts < max_attempts => {
                    attempts += 1;
                    debug!("UDS connection attempt {} failed, retrying...", attempts);
                    tokio::time::sleep(Duration::from_millis(100 * attempts)).await;
                }
                Err(e) => {
                    error!("Failed to connect to UDS after {} attempts: {}", max_attempts, e);
                    return Err(anyhow::anyhow!("UDS connection failed: {}", e));
                }
            }
        }
    }

    fn create_chat_request(&self, request: ChatCompletionRequest) -> Result<String> {
        let request_body = serde_json::to_string(&request)
            .map_err(|e| {
                error!("Failed to serialize chat request: {}", e);
                LiteLLMError::Serialization(e.to_string())
            })?;

        Ok(format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
             Host: localhost\r\n\
             User-Agent: dcmt-litellm-client/1.0\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Accept: application/json\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            request_body.len(),
            request_body
        ))
    }

    fn create_streaming_chat_request(&self, request: ChatCompletionRequest) -> Result<String> {
        let mut streaming_request = request;
        streaming_request.stream = Some(true);

        let request_body = serde_json::to_string(&streaming_request)
            .map_err(|e| {
                error!("Failed to serialize streaming chat request: {}", e);
                LiteLLMError::Serialization(e.to_string())
            })?;

        Ok(format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
             Host: localhost\r\n\
             User-Agent: dcmt-litellm-client/1.0\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Accept: text/event-stream\r\n\
             Cache-Control: no-cache\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            request_body.len(),
            request_body
        ))
    }

    async fn send_request(&self, socket: &mut UnixStream, request: &str) -> Result<()> {
        socket.write_all(request.as_bytes())?;
        socket.flush()?;
        Ok(())
    }

    async fn read_response(&self, socket: &mut UnixStream) -> Result<String> {
        let mut buffer = Vec::new();
        let mut temp_buffer = [0; 1024];

        loop {
            match socket.read(&mut temp_buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => buffer.extend_from_slice(&temp_buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                Err(_e) => {
                    error!("Failed to read from UDS");
                    return Err(anyhow::anyhow!("UDS read error"));
                }
            }
        }

        let response = String::from_utf8(buffer)?;

        // Extract JSON body from HTTP response
        let body_start = response.find("\r\n\r\n")
            .ok_or_else(|| anyhow::anyhow!("Invalid HTTP response"))?;

        let body = response[body_start + 4..].trim();

        if body.is_empty() {
            return Err(anyhow::anyhow!("Empty response body"));
        }

        Ok(body.to_string())
    }
}