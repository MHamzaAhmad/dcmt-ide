//! Web-specific editor components and implementations

pub mod codemirror_web;
pub mod editor_pane;

pub use codemirror_web::WebCodeMirrorEditor;
pub use editor_pane::{WebEditorPane, EditorViewMode};