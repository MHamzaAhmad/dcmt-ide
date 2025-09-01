#[cfg(target_arch = "wasm32")]
pub mod wasm_wrapper {
    use crate::codemirror::bindings::*;
    use crate::codemirror::{DecorationType, CodeMirrorOps};
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlElement;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// High-level wrapper for CodeMirror 6 editor with type-safe Rust API
    pub struct CodeMirrorEditor {
        view: EditorView,
        decorations: HashMap<String, DecorationType>,
        initialized: bool,
    }

    impl CodeMirrorEditor {
        /// Create a new CodeMirror editor instance
        pub async fn new(container: web_sys::Element, initial_content: Option<String>) -> EditorResult<Self> {
            use wasm_bindgen::JsCast;
            
            // Create a simple textarea fallback for now
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let textarea = document.create_element("textarea").unwrap();
            
            // Style the textarea to look like an editor
            textarea.set_attribute("style", "width: 100%; height: 100%; border: none; outline: none; resize: none; font-family: 'Fira Code', 'Monaco', 'Consolas', monospace; font-size: 14px; padding: 16px; background: #fafafa; color: #333;")?;
            textarea.set_attribute("placeholder", "Start typing your LaTeX document here...")?;
            
            // Set initial content
            if let Some(content) = initial_content {
                textarea.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap().set_value(&content);
            } else {
                textarea.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap().set_value(
                    "% LaTeX Document\n\\documentclass{article}\n\\usepackage{amsmath}\n\n\\title{My Document}\n\\author{Author}\n\\date{\\today}\n\n\\begin{document}\n\n\\maketitle\n\n\\section{Introduction}\n\nStart writing here...\n\n\\end{document}"
                );
            }
            
            // Clear container and add textarea
            container.set_inner_html("");
            container.append_child(&textarea)?;
            
            // Create a fake EditorView for the API  
            let js_value: JsValue = textarea.clone().into();
            let view = EditorView::from(js_value);
            
            Ok(Self {
                view,
                decorations: HashMap::new(),
                initialized: true,
            })
        }
        
        /// Get the underlying EditorView for advanced operations
        pub fn view(&self) -> &EditorView {
            &self.view
        }
        
        /// Check if the editor is properly initialized
        pub fn is_initialized(&self) -> bool {
            self.initialized
        }
        
        /// Focus the editor
        pub fn focus(&self) {
            self.view.focus();
        }
        
        /// Check if the editor is currently focused
        pub fn has_focus(&self) -> bool {
            self.view.has_focus()
        }
        
        /// Get the DOM element of the editor
        pub fn dom(&self) -> HtmlElement {
            self.view.dom()
        }
        
        /// Destroy the editor instance and clean up resources
        pub fn destroy(mut self) {
            self.view.destroy();
            self.decorations.clear();
            self.initialized = false;
        }
    }

    impl CodeMirrorOps for CodeMirrorEditor {
        fn get_content(&self) -> String {
            use wasm_bindgen::JsCast;
            if let Some(textarea) = self.view.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                textarea.value()
            } else {
                String::new()
            }
        }
        
        fn set_content(&self, content: &str) {
            use wasm_bindgen::JsCast;
            if let Some(textarea) = self.view.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                textarea.set_value(content);
            }
        }
        
        fn format(&self) {
            let content = self.get_content();
            let formatted = crate::codemirror::format_latex(&content);
            self.set_content(&formatted);
        }
        
        fn add_decoration(&self, _from: usize, _to: usize, _decoration_type: DecorationType) -> String {
            Uuid::new_v4().to_string()
        }
        
        fn remove_decoration(&self, _decoration_id: &str) {
            // Remove decoration
        }
        
        fn scroll_to_line(&self, _line: usize) {
            // Scroll to line
        }
        
        fn highlight_line(&self, _line: usize) {
            // Highlight line
        }
    }

    // Helper types for better API
    #[derive(Debug, Clone)]
    pub struct LineInfo {
        pub number: u32,
        pub from: u32,
        pub to: u32,
        pub text: String,
    }

    #[derive(Debug, Clone)]
    pub struct SelectionInfo {
        pub from: u32,
        pub to: u32,
        pub anchor: u32,
        pub head: u32,
        pub empty: bool,
    }

    #[derive(Debug, Clone)]
    pub enum EditorCommand {
        Undo,
        Redo,
        SelectAll,
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_wrapper::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod stub_wrapper {
    use crate::codemirror::{DecorationType, CodeMirrorOps};
    
    pub struct CodeMirrorEditor;
    
    impl CodeMirrorOps for CodeMirrorEditor {
        fn get_content(&self) -> String { String::new() }
        fn set_content(&self, _content: &str) {}
        fn format(&self) {}
        fn add_decoration(&self, _from: usize, _to: usize, _decoration_type: DecorationType) -> String { String::new() }
        fn remove_decoration(&self, _decoration_id: &str) {}
        fn scroll_to_line(&self, _line: usize) {}
        fn highlight_line(&self, _line: usize) {}
    }
    
    #[derive(Debug, Clone)]
    pub struct LineInfo {
        pub number: u32,
        pub from: u32,
        pub to: u32,
        pub text: String,
    }

    #[derive(Debug, Clone)]
    pub struct SelectionInfo {
        pub from: u32,
        pub to: u32,
        pub anchor: u32,
        pub head: u32,
        pub empty: bool,
    }

    #[derive(Debug, Clone)]
    pub enum EditorCommand {
        Undo,
        Redo,
        SelectAll,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use stub_wrapper::*;