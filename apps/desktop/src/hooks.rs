use dioxus::prelude::*;
use latex_ide_yrs_collab::{CollaborationEngine, LaTeXDocument, UserInfo};
use latex_ide_model_manager::ModelManager;
use std::sync::Arc;
use uuid::Uuid;

/// Hook for managing Yrs collaboration
pub fn use_yrs_document() -> Signal<Option<LaTeXDocument>> {
    let document = use_signal(|| None);
    
    // Initialize with a default document
    use_effect(move || {
        let user_info = UserInfo {
            id: Uuid::new_v4(),
            name: "Desktop User".to_string(),
            color: "#3B82F6".to_string(),
            cursor_position: None,
            selection_start: None,
            selection_end: None,
        };
        
        let (engine, _receiver) = CollaborationEngine::new(user_info);
        let doc_id = engine.create_document(Some(include_str!("../assets/template.tex")));
        
        if let Some(doc) = engine.get_document(&doc_id) {
            document.set(Some(doc));
        }
    });
    
    document
}

/// Hook for managing model selection and AI interactions
pub fn use_ai_models() -> (Signal<Vec<String>>, Signal<Option<String>>) {
    let available_models = use_signal(|| vec![
        "Llama 3.1 (Local)".to_string(),
        "GPT-4o (OpenAI)".to_string(),
        "Claude 3.5 Sonnet".to_string(),
        "Gemini Pro".to_string(),
    ]);
    
    let selected_model = use_signal(|| Some("Llama 3.1 (Local)".to_string()));
    
    (available_models, selected_model)
}

/// Hook for file operations with native dialogs
pub fn use_file_operations() -> FileOperations {
    FileOperations::new()
}

pub struct FileOperations;

impl FileOperations {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn open_file_dialog(&self) -> Option<String> {
        use rfd::AsyncFileDialog;
        
        let file = AsyncFileDialog::new()
            .add_filter("LaTeX files", &["tex", "latex"])
            .add_filter("All files", &["*"])
            .pick_file()
            .await;
            
        file.map(|f| f.path().to_string_lossy().to_string())
    }
    
    pub async fn save_file_dialog(&self) -> Option<String> {
        use rfd::AsyncFileDialog;
        
        let file = AsyncFileDialog::new()
            .add_filter("LaTeX files", &["tex"])
            .save_file()
            .await;
            
        file.map(|f| f.path().to_string_lossy().to_string())
    }
    
    pub async fn open_folder_dialog(&self) -> Option<String> {
        use rfd::AsyncFileDialog;
        
        let folder = AsyncFileDialog::new()
            .pick_folder()
            .await;
            
        folder.map(|f| f.path().to_string_lossy().to_string())
    }
}

/// Hook for managing window state and desktop integration
pub fn use_window_state() -> WindowState {
    WindowState::new()
}

pub struct WindowState;

impl WindowState {
    pub fn new() -> Self {
        Self
    }
    
    pub fn set_title(&self, title: &str) {
        // TODO: Update window title
        tracing::info!("Setting window title: {}", title);
    }
    
    pub fn show_notification(&self, title: &str, body: &str) {
        // TODO: Show desktop notification
        tracing::info!("Notification: {} - {}", title, body);
    }
    
    pub fn request_attention(&self) {
        // TODO: Request user attention (taskbar flash, etc.)
        tracing::info!("Requesting user attention");
    }
}