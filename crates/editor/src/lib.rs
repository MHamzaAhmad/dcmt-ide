// Re-export common dioxus types for convenience
pub use latex_ide_ui::*;

pub mod codemirror;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use codemirror::{CodeMirrorProps, CodeMirrorOps, DecorationType};