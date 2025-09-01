use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use ywasm::*;
use std::collections::HashMap;
use uuid::Uuid;
use crate::hooks::{use_local_storage, BrowserCapabilities, ConnectionState};

/// Web application state with persistence
#[derive(Clone)]
pub struct WebAppState {
    pub ui_state: Signal<WebUIState>,
    pub documents: Signal<HashMap<String, YDoc>>,
    pub active_document: Signal<Option<String>>,
    pub connection_state: Signal<ConnectionState>,
    pub capabilities: Signal<BrowserCapabilities>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebUIState {
    pub sidebar_collapsed: bool,
    pub sidebar_width: u32,
    pub editor_split: f32,
    pub ai_chat_visible: bool,
    pub pdf_preview_visible: bool,
    pub current_theme: WebTheme,
    pub zoom_level: f32,
    pub auto_compile: bool,
    pub vim_mode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebTheme {
    Light,
    Dark,
    Auto, // Follows system preference
}

impl Default for WebUIState {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            sidebar_width: 250,
            editor_split: 60.0,
            ai_chat_visible: true,
            pdf_preview_visible: true,
            current_theme: WebTheme::Auto,
            zoom_level: 1.0,
            auto_compile: false,
            vim_mode: false,
        }
    }
}

impl WebAppState {
    pub fn new() -> Self {
        // Load UI state from localStorage
        let (ui_state, save_ui_state) = use_local_storage("latex-ide-ui-state", WebUIState::default());
        
        // Auto-save UI state changes
        use_effect({
            let save_fn = save_ui_state.clone();
            move || {
                let state = ui_state.read().clone();
                save_fn(state);
            }
        });
        
        Self {
            ui_state,
            documents: use_signal(|| HashMap::new()),
            active_document: use_signal(|| None),
            connection_state: use_signal(|| ConnectionState::Disconnected),
            capabilities: use_signal(|| BrowserCapabilities {
                webtransport_supported: false,
                websocket_supported: false,
                server_sent_events: false,
                web_assembly: true,
                local_storage: false,
                indexed_db: false,
            }),
        }
    }
    
    pub fn create_document(&self, name: String, content: Option<String>) -> String {
        let doc = YDoc::new();
        let text = doc.get_text("content");
        
        if let Some(initial_content) = content {
            text.insert(0, &initial_content);
        }
        
        self.documents.write().insert(name.clone(), doc);
        self.active_document.set(Some(name.clone()));
        
        name
    }
    
    pub fn get_active_document(&self) -> Option<YDoc> {
        if let Some(doc_name) = self.active_document.read().as_ref() {
            self.documents.read().get(doc_name).cloned()
        } else {
            None
        }
    }
    
    pub fn get_document_content(&self, doc_name: &str) -> Option<String> {
        if let Some(doc) = self.documents.read().get(doc_name) {
            let text = doc.get_text("content");
            Some(text.to_string())
        } else {
            None
        }
    }
    
    pub fn update_document_content(&self, doc_name: &str, content: &str) {
        if let Some(doc) = self.documents.read().get(doc_name) {
            let text = doc.get_text("content");
            
            // Clear existing content and insert new content
            // In a real implementation, you'd want to use proper diff/patch
            let current_length = text.len();
            if current_length > 0 {
                text.remove_range(0, current_length);
            }
            text.insert(0, content);
        }
    }
    
    pub fn close_document(&self, doc_name: &str) {
        self.documents.write().remove(doc_name);
        
        // If this was the active document, switch to another one
        if self.active_document.read().as_ref() == Some(&doc_name.to_string()) {
            let next_doc = self.documents.read().keys().next().cloned();
            self.active_document.set(next_doc);
        }
    }
    
    pub fn toggle_sidebar(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.sidebar_collapsed = !ui_state.sidebar_collapsed;
    }
    
    pub fn toggle_ai_chat(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.ai_chat_visible = !ui_state.ai_chat_visible;
    }
    
    pub fn toggle_pdf_preview(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.pdf_preview_visible = !ui_state.pdf_preview_visible;
    }
    
    pub fn set_theme(&self, theme: WebTheme) {
        let mut ui_state = self.ui_state.write();
        ui_state.current_theme = theme;
        
        // Apply theme to document
        self.apply_theme_to_document();
    }
    
    pub fn toggle_theme(&self) {
        let current_theme = &self.ui_state.read().current_theme;
        let new_theme = match current_theme {
            WebTheme::Light => WebTheme::Dark,
            WebTheme::Dark => WebTheme::Light,
            WebTheme::Auto => WebTheme::Light, // Default to light when toggling from auto
        };
        self.set_theme(new_theme);
    }
    
    pub fn set_zoom_level(&self, zoom: f32) {
        let mut ui_state = self.ui_state.write();
        ui_state.zoom_level = zoom.clamp(0.5, 3.0);
        
        // Apply zoom to editor and preview
        self.apply_zoom_to_ui();
    }
    
    pub fn zoom_in(&self) {
        let current_zoom = self.ui_state.read().zoom_level;
        self.set_zoom_level(current_zoom * 1.1);
    }
    
    pub fn zoom_out(&self) {
        let current_zoom = self.ui_state.read().zoom_level;
        self.set_zoom_level(current_zoom / 1.1);
    }
    
    pub fn reset_zoom(&self) {
        self.set_zoom_level(1.0);
    }
    
    pub fn toggle_auto_compile(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.auto_compile = !ui_state.auto_compile;
    }
    
    pub fn toggle_vim_mode(&self) {
        let mut ui_state = self.ui_state.write();
        ui_state.vim_mode = !ui_state.vim_mode;
    }
    
    fn apply_theme_to_document(&self) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(html) = document.document_element() {
                    let theme_class = match self.ui_state.read().current_theme {
                        WebTheme::Light => "",
                        WebTheme::Dark => "dark",
                        WebTheme::Auto => {
                            // Check system preference
                            if window.match_media("(prefers-color-scheme: dark)").is_ok() {
                                "dark"
                            } else {
                                ""
                            }
                        }
                    };
                    
                    let _ = html.set_class_name(theme_class);
                }
            }
        }
    }
    
    fn apply_zoom_to_ui(&self) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(root) = document.get_element_by_id("app") {
                    let zoom = self.ui_state.read().zoom_level;
                    let style = format!("zoom: {}", zoom);
                    let _ = root.set_attribute("style", &style);
                }
            }
        }
    }
    
    pub fn export_state(&self) -> Result<String, serde_json::Error> {
        let state = WebAppExportState {
            ui_state: self.ui_state.read().clone(),
            document_names: self.documents.read().keys().cloned().collect(),
            active_document: self.active_document.read().clone(),
        };
        
        serde_json::to_string(&state)
    }
    
    pub fn import_state(&self, state_json: &str) -> Result<(), serde_json::Error> {
        let state: WebAppExportState = serde_json::from_str(state_json)?;
        
        // Restore UI state
        self.ui_state.set(state.ui_state);
        
        // Restore active document
        self.active_document.set(state.active_document);
        
        // Apply theme and zoom
        self.apply_theme_to_document();
        self.apply_zoom_to_ui();
        
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct WebAppExportState {
    ui_state: WebUIState,
    document_names: Vec<String>,
    active_document: Option<String>,
}

/// Project management for web app
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebProject {
    pub id: String,
    pub name: String,
    pub documents: Vec<String>,
    pub main_document: Option<String>,
    pub settings: ProjectSettings,
    pub created_at: f64,
    pub modified_at: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub latex_engine: String, // pdflatex, xelatex, lualatex
    pub bibliography_tool: String, // bibtex, biber
    pub auto_compile: bool,
    pub spell_check: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
    pub font_size: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            latex_engine: "pdflatex".to_string(),
            bibliography_tool: "bibtex".to_string(),
            auto_compile: false,
            spell_check: true,
            word_wrap: true,
            tab_size: 2,
            font_size: 14,
        }
    }
}

impl WebProject {
    pub fn new(name: String) -> Self {
        let now = js_sys::Date::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            documents: vec!["main.tex".to_string()],
            main_document: Some("main.tex".to_string()),
            settings: ProjectSettings::default(),
            created_at: now,
            modified_at: now,
        }
    }
    
    pub fn add_document(&mut self, name: String) {
        if !self.documents.contains(&name) {
            self.documents.push(name);
            self.modified_at = js_sys::Date::now();
        }
    }
    
    pub fn remove_document(&mut self, name: &str) {
        self.documents.retain(|d| d != name);
        
        // Clear main document if it was removed
        if self.main_document.as_ref() == Some(&name.to_string()) {
            self.main_document = self.documents.first().cloned();
        }
        
        self.modified_at = js_sys::Date::now();
    }
    
    pub fn set_main_document(&mut self, name: String) {
        if self.documents.contains(&name) {
            self.main_document = Some(name);
            self.modified_at = js_sys::Date::now();
        }
    }
}