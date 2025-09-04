#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings {
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlElement;
    use serde::{Deserialize, Serialize};

    #[wasm_bindgen]
    extern "C" {
        // Core CodeMirror types
        #[wasm_bindgen(js_name = EditorView, js_namespace = ["window", "CM6_EditorView"])]
        pub type EditorView;
        
        #[wasm_bindgen(js_name = EditorState, js_namespace = ["window", "CM6_EditorState"])]
        pub type EditorState;
        
        #[wasm_bindgen(js_name = Transaction, js_namespace = ["window", "CM6_Transaction"])]
        pub type Transaction;
        
        #[wasm_bindgen(js_name = StateEffect, js_namespace = ["window", "CM6_StateEffect"])]
        pub type StateEffect;
        
        #[wasm_bindgen(js_name = Compartment, js_namespace = ["window", "CM6_Compartment"])]
        pub type Compartment;
        
        // Extension types for line numbers and other features
        #[wasm_bindgen(js_name = Extension, js_namespace = ["window"])]
        pub type Extension;
    }

    #[wasm_bindgen]
    extern "C" {
        // EditorView methods
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
        
        // Global function references
        #[wasm_bindgen(js_name = CM6_basicSetup, js_namespace = ["window"])]
        pub fn basic_setup() -> Extension;
        
        #[wasm_bindgen(js_name = CM6_lineNumbers, js_namespace = ["window"])]
        pub fn line_numbers() -> Extension;
        
        #[wasm_bindgen(js_name = CM6_latex, js_namespace = ["window"])]
        pub fn latex() -> Extension;
        
        #[wasm_bindgen(js_name = CM6_oneDark, js_namespace = ["window"])]
        pub fn one_dark() -> Extension;
        
        #[wasm_bindgen(js_name = CM6_lineWrapping, js_namespace = ["window"])]
        pub fn line_wrapping() -> Extension;
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