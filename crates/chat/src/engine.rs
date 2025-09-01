//! Core chat engine logic

use crate::message::{ChatMessage, MessageRole};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl ModelConfig {
    pub fn new(id: String, name: String, provider: String) -> Self {
        Self {
            id,
            name,
            provider,
            max_tokens: Some(4000),
            temperature: Some(0.7),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatEngine {
    pub messages: Vec<ChatMessage>,
    pub available_models: Vec<ModelConfig>,
    pub selected_model: Option<String>,
    pub is_streaming: bool,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            available_models: Self::default_models(),
            selected_model: Some("gpt-4o".to_string()),
            is_streaming: false,
        }
    }
    
    fn default_models() -> Vec<ModelConfig> {
        vec![
            ModelConfig::new(
                "gpt-4o".to_string(),
                "GPT-4o (OpenAI)".to_string(),
                "openai".to_string(),
            ),
            ModelConfig::new(
                "claude-sonnet".to_string(),
                "Claude 3.5 Sonnet".to_string(),
                "anthropic".to_string(),
            ),
            ModelConfig::new(
                "gemini-pro".to_string(),
                "Gemini Pro".to_string(),
                "google".to_string(),
            ),
            ModelConfig::new(
                "llama-3.1".to_string(),
                "Llama 3.1 (via Ollama)".to_string(),
                "ollama".to_string(),
            ),
        ]
    }
    
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }
    
    pub fn add_user_message(&mut self, content: String) {
        let message = ChatMessage::new_user_message(content);
        self.add_message(message);
    }
    
    pub fn add_assistant_message(&mut self, content: String) {
        let model = self.get_selected_model();
        let message = ChatMessage::new_assistant_message(
            content, 
            model.map(|m| m.name.clone())
        );
        self.add_message(message);
    }
    
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }
    
    pub fn get_selected_model(&self) -> Option<&ModelConfig> {
        self.selected_model.as_ref().and_then(|id| {
            self.available_models.iter().find(|m| m.id == *id)
        })
    }
    
    pub fn set_selected_model(&mut self, model_id: String) {
        if self.available_models.iter().any(|m| m.id == model_id) {
            self.selected_model = Some(model_id);
        }
    }
    
    pub fn set_streaming(&mut self, streaming: bool) {
        self.is_streaming = streaming;
    }
    
    /// Get messages in a format suitable for API calls
    pub fn get_conversation_context(&self) -> Vec<&ChatMessage> {
        // Filter out system messages for most APIs, or handle them specially
        self.messages.iter()
            .filter(|msg| !matches!(msg.role, MessageRole::System))
            .collect()
    }
    
    /// Get the last user message for context
    pub fn get_last_user_message(&self) -> Option<&ChatMessage> {
        self.messages.iter()
            .rev()
            .find(|msg| msg.is_user())
    }
}