// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;

pub mod file;
pub mod tree;
pub mod project;
pub mod fs_backend;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use file::{FileItem, FileType, GitFileStatus};
pub use tree::FileTree;
pub use project::{ProjectManager, Project};
pub use fs_backend::{FileSystemBackend, FileSystemError, FileSystemResult, NativeFileSystem};