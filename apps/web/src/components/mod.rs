// Web-specific components
pub mod editor;
pub mod file_browser;
pub mod pdf_viewer;
pub mod ai_chat;
pub mod project_manager;
pub mod settings;

// Re-export for convenience
pub use editor::*;
pub use file_browser::*;
pub use pdf_viewer::*;
pub use ai_chat::*;
pub use project_manager::*;
pub use settings::*;