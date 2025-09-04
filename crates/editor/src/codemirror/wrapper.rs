#[cfg(target_arch = "wasm32")]
pub mod wasm_wrapper {
    use crate::codemirror::bindings::*;
    use crate::codemirror::{DecorationType, CodeMirrorOps, EditorConfig};
    use wasm_bindgen::prelude::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// High-level wrapper for CodeMirror 6 editor with type-safe Rust API
    pub struct CodeMirrorEditor {
        view: EditorView,
        decorations: HashMap<String, DecorationType>,
        initialized: bool,
        config: EditorConfig,
    }

    impl CodeMirrorEditor {
        /// Create a new CodeMirror editor instance
        pub async fn new(container: web_sys::Element, initial_content: Option<String>) -> EditorResult<Self> {
            Self::new_with_config(container, initial_content, EditorConfig::default()).await
        }

        /// Create a new CodeMirror editor instance with custom configuration - REAL CODEMIRROR
        pub async fn new_with_config(
            container: web_sys::Element, 
            initial_content: Option<String>,
            config: EditorConfig
        ) -> EditorResult<Self> {
            use wasm_bindgen::JsCast;
            
            // Ensure we have an HTMLElement, not just an Element
            let html_element = container.dyn_into::<web_sys::HtmlElement>()
                .map_err(|_| create_editor_error("Container must be an HTMLElement".to_string()))?;
            
            let window = web_sys::window()
                .ok_or_else(|| create_editor_error("No window object available".to_string()))?;
            
            // Check if CodeMirror 6 is loaded
            let cm6_available = js_sys::Reflect::has(&window, &JsValue::from_str("CM6"))
                .map_err(|e| EditorError::from(e))?;
                
            if !cm6_available {
                return Err(create_editor_error("CodeMirror 6 not loaded".to_string()));
            }
            
            // Get CM6 namespace
            let cm6 = js_sys::Reflect::get(&window, &JsValue::from_str("CM6"))
                .map_err(|e| EditorError::from(e))?;
            
            // Build extensions array
            let extensions = js_sys::Array::new();
            
            // Add basic setup extensions - includes line numbers by default
            if let Ok(basic_setup_fn) = js_sys::Reflect::get(&cm6, &JsValue::from_str("basicSetup")) {
                if let Ok(basic_setup) = js_sys::Reflect::apply(
                    &basic_setup_fn.dyn_into::<js_sys::Function>().map_err(|e| EditorError::from(e))?,
                    &JsValue::NULL,
                    &js_sys::Array::new()
                ) {
                    // basic_setup returns an array, spread it into our extensions
                    if let Ok(basic_array) = basic_setup.dyn_into::<js_sys::Array>() {
                        for i in 0..basic_array.length() {
                            extensions.push(&basic_array.get(i));
                        }
                    }
                }
            }
            
            // Add LaTeX language support if available
            if let Ok(latex_fn) = js_sys::Reflect::get(&cm6, &JsValue::from_str("latex")) {
                if !latex_fn.is_null() {
                    if let Ok(latex_ext) = js_sys::Reflect::apply(
                        &latex_fn.dyn_into::<js_sys::Function>().map_err(|e| EditorError::from(e))?,
                        &JsValue::NULL,
                        &js_sys::Array::new()
                    ) {
                        extensions.push(&latex_ext);
                    }
                }
            }
            
            // Note: We don't add the OneDark theme here anymore
            // Instead, we rely on CSS classes to handle theming
            // This ensures our custom zinc theme is used consistently
            
            // Create editor state
            let content = initial_content.unwrap_or_else(|| {
                "% Welcome to LaTeX IDE\n% Select a file from the workspace to start editing\n\\documentclass{article}\n\\usepackage{amsmath}\n\n\\title{My Document}\n\\author{Author}\n\\date{\\today}\n\n\\begin{document}\n\n\\maketitle\n\n\\section{Introduction}\n\nStart writing your LaTeX document here...\n\n\\end{document}".to_string()
            });
            
            let state_config = js_sys::Object::new();
            js_sys::Reflect::set(&state_config, &JsValue::from_str("doc"), &JsValue::from_str(&content))
                .map_err(|e| EditorError::from(e))?;
            js_sys::Reflect::set(&state_config, &JsValue::from_str("extensions"), &extensions)
                .map_err(|e| EditorError::from(e))?;
            
            // Create EditorState using CM6.EditorState.create()
            let editor_state_class = js_sys::Reflect::get(&cm6, &JsValue::from_str("EditorState"))
                .map_err(|e| EditorError::from(e))?;
            let create_fn = js_sys::Reflect::get(&editor_state_class, &JsValue::from_str("create"))
                .map_err(|e| EditorError::from(e))?;
            let editor_state = js_sys::Reflect::apply(
                &create_fn.dyn_into::<js_sys::Function>().map_err(|e| EditorError::from(e))?,
                &editor_state_class,
                &js_sys::Array::of1(&state_config)
            ).map_err(|e| EditorError::from(e))?;
            
            // Create EditorView using new CM6.EditorView()
            let view_config = js_sys::Object::new();
            js_sys::Reflect::set(&view_config, &JsValue::from_str("state"), &editor_state)
                .map_err(|e| EditorError::from(e))?;
            js_sys::Reflect::set(&view_config, &JsValue::from_str("parent"), &html_element)
                .map_err(|e| EditorError::from(e))?;
            
            let editor_view_class = js_sys::Reflect::get(&cm6, &JsValue::from_str("EditorView"))
                .map_err(|e| EditorError::from(e))?;
            let editor_view = js_sys::Reflect::construct(
                &editor_view_class.dyn_into::<js_sys::Function>().map_err(|e| EditorError::from(e))?,
                &js_sys::Array::of1(&view_config)
            ).map_err(|e| EditorError::from(e))?;
            
            let view = EditorView::from(editor_view);
            
            Ok(Self {
                view,
                decorations: HashMap::new(),
                initialized: true,
                config,
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
            if let Ok(focus_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("focus")) {
                js_sys::Reflect::apply(
                    &focus_method.dyn_into::<js_sys::Function>().unwrap_or_else(|_| js_sys::Function::new_no_args("")),
                    &self.view,
                    &js_sys::Array::new()
                ).ok();
            }
        }
        
        /// Check if the editor is currently focused
        pub fn has_focus(&self) -> bool {
            if let Ok(has_focus_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("hasFocus")) {
                if let Ok(result) = js_sys::Reflect::apply(
                    &has_focus_method.dyn_into::<js_sys::Function>().unwrap_or_else(|_| js_sys::Function::new_no_args("")),
                    &self.view,
                    &js_sys::Array::new()
                ) {
                    return result.as_bool().unwrap_or(false);
                }
            }
            false
        }
        
        /// Get the DOM element of the editor
        pub fn dom(&self) -> web_sys::HtmlElement {
            if let Ok(dom_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("dom")) {
                if let Ok(dom_element) = js_sys::Reflect::get(&self.view, &JsValue::from_str("dom")) {
                    use wasm_bindgen::JsCast;
                    if let Ok(element) = dom_element.dyn_into::<web_sys::HtmlElement>() {
                        return element;
                    }
                }
            }
            
            // Fallback - create a dummy element
            use wasm_bindgen::JsCast;
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            document.create_element("div").unwrap().dyn_into::<web_sys::HtmlElement>().unwrap()
        }
        
        /// Destroy the editor instance and clean up resources
        pub fn destroy(mut self) {
            if let Ok(destroy_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("destroy")) {
                js_sys::Reflect::apply(
                    &destroy_method.dyn_into::<js_sys::Function>().unwrap_or_else(|_| js_sys::Function::new_no_args("")),
                    &self.view,
                    &js_sys::Array::new()
                ).ok();
            }
            self.decorations.clear();
            self.initialized = false;
        }
    }

    impl CodeMirrorOps for CodeMirrorEditor {
        fn get_content(&self) -> String {
            // Get content from real CodeMirror state
            if let Ok(state_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("state")) {
                if let Ok(state) = js_sys::Reflect::get(&self.view, &JsValue::from_str("state")) {
                    if let Ok(doc) = js_sys::Reflect::get(&state, &JsValue::from_str("doc")) {
                        if let Ok(to_string_method) = js_sys::Reflect::get(&doc, &JsValue::from_str("toString")) {
                            if let Ok(content) = js_sys::Reflect::apply(
                                &to_string_method.dyn_into::<js_sys::Function>().unwrap_or_else(|_| js_sys::Function::new_no_args("")),
                                &doc,
                                &js_sys::Array::new()
                            ) {
                                return content.as_string().unwrap_or_default();
                            }
                        }
                    }
                }
            }
            String::new()
        }
        
        fn set_content(&self, content: &str) {
            // Set content using CodeMirror transaction - this is more complex
            // For now, we can create a new state and replace it
            if let Ok(window) = web_sys::window().ok_or("No window") {
                if let Ok(cm6) = js_sys::Reflect::get(&window, &JsValue::from_str("CM6")) {
                    // Create a simple change transaction
                    let state = js_sys::Reflect::get(&self.view, &JsValue::from_str("state")).unwrap_or(JsValue::NULL);
                    let doc_length = if let Ok(doc) = js_sys::Reflect::get(&state, &JsValue::from_str("doc")) {
                        js_sys::Reflect::get(&doc, &JsValue::from_str("length")).unwrap_or(JsValue::from_f64(0.0))
                    } else {
                        JsValue::from_f64(0.0)
                    };
                    
                    // Create a change object that replaces the entire document
                    let change = js_sys::Object::new();
                    js_sys::Reflect::set(&change, &JsValue::from_str("from"), &JsValue::from_f64(0.0)).ok();
                    js_sys::Reflect::set(&change, &JsValue::from_str("to"), &doc_length).ok();
                    js_sys::Reflect::set(&change, &JsValue::from_str("insert"), &JsValue::from_str(content)).ok();
                    
                    // Create transaction with the change
                    let transaction = js_sys::Object::new();
                    js_sys::Reflect::set(&transaction, &JsValue::from_str("changes"), &js_sys::Array::of1(&change)).ok();
                    
                    // Dispatch the transaction
                    if let Ok(dispatch_method) = js_sys::Reflect::get(&self.view, &JsValue::from_str("dispatch")) {
                        js_sys::Reflect::apply(
                            &dispatch_method.dyn_into::<js_sys::Function>().unwrap_or_else(|_| js_sys::Function::new_no_args("")),
                            &self.view,
                            &js_sys::Array::of1(&transaction)
                        ).ok();
                    }
                }
            }
        }
        
        fn format(&self) {
            let content = self.get_content();
            let formatted = crate::codemirror::format_latex(&content);
            self.set_content(&formatted);
        }
        
        fn add_decoration(&self, from: usize, to: usize, decoration_type: DecorationType) -> String {
            let decoration_id = format!("decoration-{}", Uuid::new_v4().simple());
            // CodeMirror decoration implementation would go here
            decoration_id
        }
        
        fn remove_decoration(&self, _decoration_id: &str) {
            // CodeMirror decoration removal would go here
        }
        
        fn highlight_line(&self, _line: usize) {
            // Highlight line implementation using CodeMirror API
        }
        
        fn scroll_to_line(&self, _line: usize) {
            // Scroll to line implementation using CodeMirror API
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

    // Error handling helpers
    use crate::codemirror::bindings::{EditorError, EditorResult, create_editor_error};
}

#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_wrapper {
    // Stub implementation for non-WASM targets
    pub use super::*;
}

pub use wasm_wrapper::*;