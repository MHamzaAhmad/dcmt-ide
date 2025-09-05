pub mod types;
pub mod client;

// Platform-specific modules
#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "desktop")]
pub mod desktop;

// Re-exports
pub use types::*;
pub use client::{LlmClient, LlmClientConfig};

#[cfg(feature = "web")]
pub use web::WebLlmClient;

#[cfg(feature = "desktop")]
pub use desktop::DesktopLlmClient;

/// Create a platform-appropriate LLM client
#[cfg(feature = "web")]
pub fn create_client(config: LlmClientConfig) -> impl LlmClient {
    WebLlmClient::new(config)
}

/// Create a platform-appropriate LLM client
#[cfg(feature = "desktop")]
pub fn create_client(config: LlmClientConfig) -> impl LlmClient {
    DesktopLlmClient::new(config)
}

/// Default client factory with default configuration
pub fn create_default_client() -> impl LlmClient {
    create_client(LlmClientConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection() {
        assert_eq!(Provider::from_model_name("gpt-4"), Provider::OpenAI);
        assert_eq!(Provider::from_model_name("claude-3-sonnet"), Provider::Anthropic);
        assert_eq!(Provider::from_model_name("gemini-pro"), Provider::Gemini);
        assert_eq!(Provider::from_model_name("unknown-model"), Provider::OpenAI);
    }

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        };
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello, world!"));
        
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "user");
        assert_eq!(deserialized.content, "Hello, world!");
    }

    #[test]
    fn test_chat_completion_request_serialization() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            tools: None,
            stream: true,
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            stop: None,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("Hello"));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("0.7"));
    }
}