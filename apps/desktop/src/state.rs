use dioxus_hooks::use_signal;
use dioxus_signals::{Signal, Readable, Writable};
use latex_ide_yrs_collab::{CollaborationEngine, LaTeXDocument};
use latex_ide_model_manager::ModelManager;
use latex_ide_file_manager::ProjectManager;
use latex_ide_version_control_ui::VersionControlState;
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;

/// Global application state for the desktop app
#[derive(Clone, PartialEq)]
pub struct AppState {
    pub project_manager: Signal<ProjectManager>,
    pub open_documents: Signal<HashMap<Uuid, LaTeXDocument>>,
    pub active_document: Signal<Option<Uuid>>,
    pub collaboration_engine: Signal<Option<Arc<CollaborationEngine>>>,
    pub model_manager: Signal<Option<Arc<ModelManager>>>,
    pub ui_state: Signal<UIState>,
    pub version_control: Signal<VersionControlState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub root_path: String,
    pub main_document: Option<Uuid>,
    pub documents: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UIState {
    pub sidebar_collapsed: bool,
    pub sidebar_width: u32,
    pub editor_split: f32,
    pub ai_chat_visible: bool,
    pub pdf_preview_visible: bool,
    pub git_panel_visible: bool,
    pub current_theme: latex_ide_ui::Theme,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            sidebar_width: 250,
            editor_split: 60.0,
            ai_chat_visible: true,
            pdf_preview_visible: true,
            git_panel_visible: false,
            current_theme: latex_ide_ui::Theme::Light,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            project_manager: use_signal(|| ProjectManager::new()),
            open_documents: use_signal(|| HashMap::new()),
            active_document: use_signal(|| None),
            collaboration_engine: use_signal(|| None),
            model_manager: use_signal(|| None),
            ui_state: use_signal(|| UIState::default()),
            version_control: use_signal(|| VersionControlState::new()),
        }
    }

    pub fn create_new_document(&mut self, content: Option<String>) -> Uuid {
        let doc_id = Uuid::new_v4();
        
        // Create LaTeX document with Yrs CRDT
        if let Some(engine) = self.collaboration_engine.read().as_ref() {
            let document_id = engine.create_document(content.as_deref());
            
            if let Some(doc) = engine.get_document(&document_id) {
                self.open_documents.write().insert(doc_id, doc);
                self.active_document.set(Some(doc_id));
            }
        }
        
        doc_id
    }

    pub fn open_document(&mut self, file_path: &str) -> Result<Uuid, std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let doc_id = self.create_new_document(Some(content));
        Ok(doc_id)
    }

    pub fn save_document(&self, doc_id: Uuid, file_path: &str) -> Result<(), std::io::Error> {
        if let Some(doc) = self.open_documents.read().get(&doc_id) {
            let content = doc.get_content();
            std::fs::write(file_path, content)?;
        }
        Ok(())
    }

    pub fn close_document(&mut self, doc_id: Uuid) {
        self.open_documents.write().remove(&doc_id);
        
        // If this was the active document, switch to another one
        if self.active_document.read().as_ref() == Some(&doc_id) {
            let next_doc = self.open_documents.read().keys().next().copied();
            self.active_document.set(next_doc);
        }
    }

    pub fn get_active_document(&self) -> Option<LaTeXDocument> {
        if let Some(doc_id) = *self.active_document.read() {
            self.open_documents.read().get(&doc_id).cloned()
        } else {
            None
        }
    }

    pub fn toggle_sidebar(&mut self) {
        let mut ui_state = self.ui_state.write();
        ui_state.sidebar_collapsed = !ui_state.sidebar_collapsed;
    }

    pub fn toggle_theme(&mut self) {
        let mut ui_state = self.ui_state.write();
        ui_state.current_theme = ui_state.current_theme.toggle();
    }

    pub fn set_editor_split(&mut self, split: f32) {
        let mut ui_state = self.ui_state.write();
        ui_state.editor_split = split.clamp(20.0, 80.0);
    }
}