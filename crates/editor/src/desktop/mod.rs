//! Desktop-specific editor components and implementations

pub mod codemirror_desktop;
pub mod editor_pane;

pub use codemirror_desktop::DesktopCodeMirrorEditor;
pub use editor_pane::{DesktopEditorPane, EditorViewMode};