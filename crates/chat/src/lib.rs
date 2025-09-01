// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;

pub mod message;
pub mod engine;

// Platform-specific modules
pub mod web;
#[cfg(feature = "desktop")]
pub mod desktop;

pub use message::{ChatMessage, MessageRole};
pub use engine::ChatEngine;