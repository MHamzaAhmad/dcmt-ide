use dioxus::prelude::*;
use dioxus_hooks::{use_resource, use_effect, Resource};
use dioxus_desktop::launch::launch;
use latex_ide_ui::*;
use latex_ide_yrs_collab::{CollaborationEngine, LaTeXDocument, UserInfo};
use latex_ide_model_manager::ModelManager;
use uuid::Uuid;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

// Import components from shared crates
use latex_ide_editor::desktop::DesktopCodeMirrorEditor;
use latex_ide_chat::desktop::DesktopAIChatInterface;
use latex_ide_file_manager::desktop::DesktopFileTree;
use latex_ide_pdf_viewer::desktop::DesktopPreviewPane;
use latex_ide_version_control_ui::{GitPanel, VersionControlProvider};

mod state;
use state::AppState;

const WINDOW_TITLE: &str = "LaTeX IDE - Desktop";

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting LaTeX IDE Desktop application");

    // Launch Dioxus desktop app  
    launch(App, vec![], vec![])
}

#[component]
fn App() -> Element {
    // Initialize application state
    let mut app_state = use_context_provider(|| AppState::new());

    // Initialize version control for current workspace
    use_effect(move || {
        let mut app_state = app_state.clone();
        spawn(async move {
            if let Some(project) = app_state.project_manager.read().get_current_project() {
                let workspace_path = project.workspace_root.clone();
                match app_state.version_control.write().initialize(workspace_path) {
                    Ok(_) => {
                        info!("Version control initialized for workspace");
                        // Start a session for this IDE instance
                        if let Some(ref session_manager) = app_state.version_control.read().session_manager {
                            if let Ok(session_branch) = session_manager.start_session() {
                                info!("Started Git session: {}", session_branch);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to initialize version control: {}", e);
                    }
                }
            }
        });
    });
    
    // Initialize collaboration engine
    let collab_engine = use_resource(move || async move {
        let user_info = UserInfo {
            id: Uuid::new_v4(),
            name: "Desktop User".to_string(),
            color: "#3B82F6".to_string(),
            cursor_position: None,
            selection_start: None,
            selection_end: None,
        };
        
        let (engine, _receiver) = CollaborationEngine::new(user_info);
        Arc::new(engine)
    });

    // Initialize model manager
    let model_manager = use_resource(move || async move {
        match ModelManager::new(Default::default()).await {
            Ok(manager) => Some(Arc::new(manager)),
            Err(e) => {
                error!("Failed to initialize model manager: {}", e);
                None
            }
        }
    });
    
    // Update app state when resources are ready
    use_effect(move || {
        if let Some(engine) = collab_engine.read().as_ref() {
            app_state.collaboration_engine.set(Some(engine.clone()));
        }
    });
    
    use_effect(move || {
        if let Some(manager) = model_manager.read().as_ref() {
            app_state.model_manager.set(manager.clone());
        }
    });

    rsx! {
        VersionControlProvider {
            ThemeProvider {
                div { 
                    id: "app",
                    class: "h-screen w-screen bg-white dark:bg-gray-900 flex flex-col",
                    
                    // Menu bar
                    MenuBar { app_state: app_state }
                    
                    // Main application layout
                    div { class: "flex-1 flex overflow-hidden relative",
                    
                    // Sidebar with file tree
                    Sidebar { 
                        collapsed: false,
                        width: 250,
                        DesktopFileTree { 
                            project_manager: app_state.project_manager,
                            onfile_select: {
                                let mut app_state = app_state.clone();
                                move |file_path: String| {
                                    info!("Selected file: {:?}", file_path);
                                    // TODO: Open the selected file in the editor
                                    let _ = app_state.open_document(&file_path);
                                }
                            }
                        }
                    }
                    
                    // Main editor area
                    div { class: "flex-1 flex",
                        SplitView {
                            initial_split: 60.0,
                            resizable: true,
                            
                            left: rsx! {
                                EditorPane {
                                    collab_engine: collab_engine,
                                }
                            },
                            
                            right: rsx! {
                                DesktopPreviewPane {}
                            }
                        }
                    }
                    
                    // AI Chat sidebar  
                    div { class: "w-80 border-l border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800",
                        if app_state.ui_state.read().ai_chat_visible {
                            DesktopAIChatInterface {
                                model_manager: app_state.model_manager.read().as_ref().map(|m| use_signal(|| Some(m.clone()))),
                            }
                        } else {
                            div { class: "h-full flex items-center justify-center text-gray-500",
                                "AI Chat Hidden"
                            }
                        }
                    }
                }
                
                    // Git Panel (overlay)
                    GitPanel {
                        visible: app_state.ui_state.read().git_panel_visible,
                        width: 400,
                        on_close: {
                            let mut app_state = app_state.clone();
                            move |_| {
                                let mut ui_state = app_state.ui_state.write();
                                ui_state.git_panel_visible = false;
                            }
                        }
                    }
                }
                
                // Status bar
                StatusBar { app_state: app_state }
                }
            }
        }
    }
}

#[component]
fn MenuBar(app_state: AppState) -> Element {
    rsx! {
        div { class: "h-8 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center px-4",
            div { class: "text-sm text-gray-600 dark:text-gray-300",
                "File | Edit | View | Tools | Help"
            }
            div { class: "flex-1" }
            
            // Git status and controls
            div { class: "flex items-center space-x-4",
                GitStatusIndicator {
                    has_changes: app_state.version_control.read().has_changes(),
                    current_branch: app_state.version_control.read().get_current_branch().unwrap_or_else(|| "main".to_string()),
                    session_branch: app_state.version_control.read().get_session_branch()
                }
                
                GitPanelToggle {
                    visible: app_state.ui_state.read().git_panel_visible,
                    has_changes: app_state.version_control.read().has_changes(),
                    on_toggle: {
                        let mut app_state = app_state.clone();
                        move |_| {
                            let mut ui_state = app_state.ui_state.write();
                            ui_state.git_panel_visible = !ui_state.git_panel_visible;
                        }
                    }
                }
                
                div { class: "text-sm text-gray-600 dark:text-gray-300",
                    "LaTeX IDE v0.1.0"
                }
            }
        }
    }
}

#[component]
fn EditorPane(collab_engine: Resource<Arc<CollaborationEngine>>) -> Element {
    let mut current_document = use_signal(|| None::<LaTeXDocument>);
    let mut document_content = use_signal(|| String::new());
    
    // Initialize document when collaboration engine is ready
    use_effect(move || {
        if let Some(engine) = collab_engine.read().as_ref() {
            let doc_id = engine.create_document(Some(r#"\documentclass{article}
\usepackage{amsmath}

\title{My LaTeX Document}
\author{User}
\date{\today}

\begin{document}

\maketitle

\section{Introduction}

Hello, world!

\end{document}"#));
            
            if let Some(doc) = engine.get_document(&doc_id) {
                current_document.set(Some(doc.clone()));
                document_content.set(doc.get_content());
            }
        }
    });
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // Document tabs
            div { class: "border-b border-gray-200 dark:border-gray-700",
                Tabs {
                    tabs: vec![
                        latex_ide_ui::Tab {
                            id: "document".to_string(),
                            label: "document.tex".to_string(),
                            content: rsx! { div {} }
                        }
                    ],
                    active_tab: Some("document".to_string()),
                    onchange: move |_| {}
                }
            }
            
            // Editor using CodeMirror
            div { class: "flex-1",
                if current_document.read().is_some() {
                    DesktopCodeMirrorEditor {
                        content: document_content,
                        enable_ai_suggestions: true,
                        enable_pdf_sync: true,
                    }
                } else {
                    div { class: "h-full flex items-center justify-center text-gray-500",
                        "Loading document..."
                    }
                }
            }
        }
    }
}

#[component]
fn StatusBar(app_state: AppState) -> Element {
    let mut theme = use_theme();
    
    rsx! {
        div { class: "h-6 bg-blue-600 text-white text-xs flex items-center justify-between px-4",
            div { class: "flex items-center space-x-4",
                span { "Ready" }
                
                // Workspace info
                span { 
                    if let Some(project) = app_state.project_manager.read().get_current_project() {
                        "📁 {project.name}"
                    } else {
                        "📁 No workspace"
                    }
                }
                
                // Main file info
                if let Some(main_file) = app_state.project_manager.read().get_current_project()
                    .and_then(|p| p.main_file.as_ref()) {
                    span { "📝 {main_file.display()}" }
                }
                
                span { "UTF-8" }
                span { "LaTeX" }
                span { 
                    if app_state.active_document.read().is_some() {
                        "Document Active"
                    } else {
                        "No document"
                    }
                }
            }
            
            div { class: "flex items-center space-x-4",
                button {
                    class: "hover:bg-blue-700 px-2 py-0.5 rounded",
                    onclick: {
                        let mut app_state = app_state.clone();
                        move |_| {
                            let mut ui_state = app_state.ui_state.write();
                            ui_state.ai_chat_visible = !ui_state.ai_chat_visible;
                        }
                    },
                    if app_state.ui_state.read().ai_chat_visible { "💬" } else { "💬" }
                }
                
                button {
                    class: "hover:bg-blue-700 px-2 py-0.5 rounded",
                    onclick: move |_| {
                        let new_theme = theme.read().toggle();
                        theme.set(new_theme);
                    },
                    match *theme.read() {
                        Theme::Light => "🌙",
                        Theme::Dark => "☀️"
                    }
                }
                
                span { 
                    if app_state.collaboration_engine.read().is_some() {
                        "🟢 Connected"
                    } else {
                        "🟡 Offline"
                    }
                }
                span { "Yrs CRDT" }
            }
        }
    }
}

// TODO: Fix menu API compatibility
// fn create_menu() -> dioxus::desktop::tao::menu::MenuBar {
//     use dioxus::desktop::tao::menu::*;
//     
//     MenuBar::new()
//         .add_submenu(Submenu::new("File", true)
//             .add_item(MenuItem::new("New", true, None))
//             .add_item(MenuItem::new("Open", true, None))
//             .add_item(MenuItem::new("Save", true, None))
//             .add_separator()
//             .add_item(MenuItem::new("Exit", true, None)))
//         .add_submenu(Submenu::new("Edit", true)
//             .add_item(MenuItem::new("Undo", true, None))
//             .add_item(MenuItem::new("Redo", true, None))
//             .add_separator()
//             .add_item(MenuItem::new("Cut", true, None))
//             .add_item(MenuItem::new("Copy", true, None))
//             .add_item(MenuItem::new("Paste", true, None)))
//         .add_submenu(Submenu::new("View", true)
//             .add_item(MenuItem::new("Toggle Theme", true, None))
//             .add_item(MenuItem::new("Toggle Sidebar", true, None)))
// }