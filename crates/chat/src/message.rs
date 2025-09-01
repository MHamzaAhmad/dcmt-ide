//! Chat message types and utilities

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    pub timestamp: f64,
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
}

impl ChatMessage {
    pub fn new_user_message(content: String) -> Self {
        Self {
            id: generate_id(),
            content,
            role: MessageRole::User,
            timestamp: get_timestamp(),
            model: None,
            tokens_used: None,
        }
    }
    
    pub fn new_assistant_message(content: String, model: Option<String>) -> Self {
        Self {
            id: generate_id(),
            content,
            role: MessageRole::Assistant,
            timestamp: get_timestamp(),
            model,
            tokens_used: None,
        }
    }
    
    pub fn new_system_message(content: String) -> Self {
        Self {
            id: generate_id(),
            content,
            role: MessageRole::System,
            timestamp: get_timestamp(),
            model: None,
            tokens_used: None,
        }
    }
    
    pub fn is_user(&self) -> bool {
        matches!(self.role, MessageRole::User)
    }
    
    pub fn is_assistant(&self) -> bool {
        matches!(self.role, MessageRole::Assistant)
    }
}

fn generate_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    get_timestamp().to_bits().hash(&mut hasher);
    format!("msg_{}", hasher.finish())
}

#[cfg(target_arch = "wasm32")]
fn get_timestamp() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_timestamp() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}