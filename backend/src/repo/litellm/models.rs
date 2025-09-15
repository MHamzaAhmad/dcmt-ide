use super::types::*;
use anyhow::Result;
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::time::Duration;
use serde_json;
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

        // Connect to UDS socket
        let mut socket = self.connect_with_retry().await?;

        // Send HTTP request to LiteLLM
        let request = self.create_models_request();
        self.send_request(&mut socket, &request).await?;

        // Read response
        let response = self.read_response(&mut socket).await?;

        // Parse as LiteLLM models response
        let models_response: LiteLLMModelsResponse = serde_json::from_str(&response)
            .map_err(|e| {
                error!("Failed to parse models response: {}", e);
                LiteLLMError::InvalidResponse(e.to_string())
            })?;

        debug!("Successfully fetched {} models", models_response.data.len());
        Ok(models_response)
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
                Err(_e) => {
                    error!("Failed to connect to UDS after {} attempts", max_attempts);
                    return Err(anyhow::anyhow!("UDS connection failed"));
                }
            }
        }
    }

    fn create_models_request(&self) -> String {
        format!(
            "GET /v1/models HTTP/1.1\r\n\
             Host: localhost\r\n\
             User-Agent: dcmt-litellm-client/1.0\r\n\
             Accept: application/json\r\n\
             Connection: close\r\n\
             \r\n"
        )
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