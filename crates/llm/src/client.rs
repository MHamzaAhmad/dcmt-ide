use crate::types::*;
use futures::stream::Stream;
use std::pin::Pin;

/// Unified LLM client trait
pub trait LlmClient {
    /// List available models for a provider
    async fn list_models(&self, provider: Option<Provider>) -> Result<ModelsResponse, LlmError>;
    
    /// Create a streaming chat completion
    async fn chat_completions_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, LlmError>>>>, LlmError>;
}

/// LLM client configuration
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub base_url: String,
    pub auth_token: String, // Empty for now as requested
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(), // Adjust based on your server
            auth_token: String::new(), // Empty as requested
        }
    }
}