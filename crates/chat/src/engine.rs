//! Core chat engine logic

use crate::message::{ChatMessage, MessageRole};
use serde::{Serialize, Deserialize};

#[cfg(any(feature = "web", feature = "desktop"))]
use latex_ide_llm::{LlmClient, Provider, create_client};

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
    pub is_loading_models: bool,
    pub model_load_error: Option<String>,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            available_models: Vec::new(),
            selected_model: None,
            is_streaming: false,
            is_loading_models: false,
            model_load_error: None,
        }
    }
    
    /// Load models from the LLM API
    #[cfg(any(feature = "web", feature = "desktop"))]
    pub async fn load_models(&mut self) -> Result<(), String> {
        self.is_loading_models = true;
        self.model_load_error = None;
        
        let client = create_client(Default::default());
        
        match client.list_models(None).await {
            Ok(response) => {
                self.available_models = response.data
                    .into_iter()
                    .map(|model| {
                        let provider = Provider::from_model_name(&model.id);
                        ModelConfig::new(
                            model.id.clone(),
                            format!("{} ({})", model.id, provider.as_str()),
                            provider.as_str().to_string(),
                        )
                    })
                    .collect();
                
                // Set default model if none selected and models are available
                if self.selected_model.is_none() && !self.available_models.is_empty() {
                    self.selected_model = Some(self.available_models[0].id.clone());
                }
                
                self.is_loading_models = false;
                tracing::info!("Loaded {} models from API", self.available_models.len());
                Ok(())
            }
            Err(e) => {
                self.is_loading_models = false;
                let error_msg = format!("Failed to load models: {}", e);
                self.model_load_error = Some(error_msg.clone());
                tracing::error!("{}", error_msg);
                
                // Fallback to default models
                self.available_models = Self::fallback_models();
                if self.selected_model.is_none() && !self.available_models.is_empty() {
                    self.selected_model = Some(self.available_models[0].id.clone());
                }
                
                Err(error_msg)
            }
        }
    }
    
    /// Fallback models when API is not available
    fn fallback_models() -> Vec<ModelConfig> {
        vec![
            ModelConfig::new(
                "gpt-4o".to_string(),
                "GPT-4o (OpenAI)".to_string(),
                "openai".to_string(),
            ),
            ModelConfig::new(
                "claude-3-5-sonnet-20241022".to_string(),
                "Claude 3.5 Sonnet (Anthropic)".to_string(),
                "anthropic".to_string(),
            ),
            ModelConfig::new(
                "gemini-1.5-pro".to_string(),
                "Gemini 1.5 Pro (Google)".to_string(),
                "gemini".to_string(),
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