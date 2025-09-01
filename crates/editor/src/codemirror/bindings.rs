#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings {
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlElement;
    use serde::{Deserialize, Serialize};

    #[wasm_bindgen]
    extern "C" {
        // Core CodeMirror types
        #[wasm_bindgen(js_name = EditorView, js_namespace = ["window", "CM6", "view"])]
        pub type EditorView;
        
        #[wasm_bindgen(js_name = EditorState, js_namespace = ["window", "CM6", "state"])]
        pub type EditorState;
        
        #[wasm_bindgen(js_name = Transaction, js_namespace = ["window", "CM6", "state"])]
        pub type Transaction;
        
        #[wasm_bindgen(js_name = StateEffect, js_namespace = ["window", "CM6", "state"])]
        pub type StateEffect;
        
        #[wasm_bindgen(js_name = Compartment, js_namespace = ["window", "CM6", "state"])]
        pub type Compartment;
    }

    #[wasm_bindgen]
    extern "C" {
        // EditorView methods
        #[wasm_bindgen(constructor, js_namespace = ["window", "CM6", "view"])]
        pub fn new_editor_view(config: &JsValue) -> EditorView;
        
        #[wasm_bindgen(method, getter)]
        pub fn state(this: &EditorView) -> EditorState;
        
        #[wasm_bindgen(method)]
        pub fn dispatch(this: &EditorView, transaction: &Transaction) -> bool;
        
        #[wasm_bindgen(method)]
        pub fn destroy(this: &EditorView);
        
        #[wasm_bindgen(method)]
        pub fn focus(this: &EditorView);
        
        #[wasm_bindgen(method, js_name = hasFocus)]
        pub fn has_focus(this: &EditorView) -> bool;
        
        #[wasm_bindgen(method, getter)]
        pub fn dom(this: &EditorView) -> HtmlElement;
        
        #[wasm_bindgen(method, js_name = contentDOM)]
        pub fn content_dom(this: &EditorView) -> HtmlElement;
        
        // Position and coordinate methods (simplified for now)
        // TODO: Implement proper coordinate handling
    }

    // Simplified types and functions for compilation
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct EditorViewConfig;
    
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CoordsObject {
        pub x: f64,
        pub y: f64,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_bindings {
    // Stub types for non-wasm compilation
    pub struct EditorView;
    pub struct EditorState;
    pub struct Transaction;
    pub struct StateEffect;
    pub struct Compartment;
    pub struct EditorViewConfig;
    pub struct CoordsObject {
        pub x: f64,
        pub y: f64,
    }
}

// Re-export for easier use
pub use wasm_bindings::*;

// Error types for better error handling
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditorError {
    pub message: String,
    pub code: Option<String>,
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Editor Error: {}", self.message)
    }
}

impl std::error::Error for EditorError {}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for EditorError {
    fn from(js_value: wasm_bindgen::JsValue) -> Self {
        EditorError {
            message: format!("JavaScript error: {:?}", js_value),
            code: None,
        }
    }
}

pub type EditorResult<T> = Result<T, EditorError>;

// Helper functions that work across platforms
pub fn create_editor_error(message: String) -> EditorError {
    EditorError {
        message,
        code: None,
    }
}