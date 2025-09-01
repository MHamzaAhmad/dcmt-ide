// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;

pub mod file;
pub mod tree;
pub mod project;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use file::{FileItem, FileType};
pub use tree::FileTree;
pub use project::{ProjectManager, Project};