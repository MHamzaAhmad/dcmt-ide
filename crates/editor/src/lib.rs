// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;
pub use dioxus_hooks::use_effect;

pub mod buffer;
pub mod editor;
pub mod syntax;
pub mod cursor;
pub mod selection;
pub mod commands;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use editor::TextEditor;
pub use buffer::TextBuffer;
pub use syntax::SyntaxHighlighter;
pub use cursor::Cursor;
pub use selection::Selection;
pub use commands::{EditorCommand, CommandExecutor};