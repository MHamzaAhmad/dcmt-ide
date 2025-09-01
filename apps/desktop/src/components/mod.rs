// Desktop-specific components
pub mod editor;
pub mod file_browser;
pub mod project;
pub mod settings;

// Re-export for convenience
pub use editor::*;
pub use file_browser::*;
pub use project::*;
pub use settings::*;