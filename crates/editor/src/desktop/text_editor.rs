use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Owner, GlobalSignal};
use crate::TextEditor;

/// Desktop text editor component that uses the shared TextEditor
#[component]
pub fn DesktopTextEditor(
    initial_content: Option<String>,
    onchange: Option<EventHandler<String>>,
    show_line_numbers: Option<bool>,
    syntax_highlighting: Option<bool>,
) -> Element {
    rsx! {
        TextEditor {
            initial_content: initial_content,
            onchange: onchange,
            show_line_numbers: show_line_numbers.unwrap_or(true),
            syntax_highlighting: syntax_highlighting.unwrap_or(true),
        }
    }
}

/// Simple desktop text editor with content signal
#[component]
pub fn SimpleDesktopTextEditor(content: Signal<String>) -> Element {
    rsx! {
        TextEditor {
            initial_content: Some(content.read().clone()),
            onchange: None, // TODO: Fix closure event handler for desktop
            show_line_numbers: true,
            syntax_highlighting: true,
        }
    }
}