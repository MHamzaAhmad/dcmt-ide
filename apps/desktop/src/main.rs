use dioxus::prelude::*;
use dioxus_hooks::{use_resource, use_effect, Resource};
use latex_ide_ui::*;
use latex_ide_yrs_collab::{CollaborationEngine, LaTeXDocument, UserInfo};
use latex_ide_model_manager::ModelManager;
use uuid::Uuid;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

// Import components from shared crates
use latex_ide_editor::desktop::DesktopTextEditor;
use latex_ide_chat::desktop::DesktopAIChatInterface;
use latex_ide_file_manager::desktop::DesktopFileTree;
use latex_ide_pdf_viewer::desktop::DesktopPreviewPane;

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
    dioxus_desktop::launch(App);

    Ok(())
}

#[component]
fn App() -> Element {
    // Initialize application state
    let _app_state = use_context_provider(|| AppState::new());
    
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

    rsx! {
        ThemeProvider {
            div { 
                id: "app",
                class: "h-screen w-screen bg-white dark:bg-gray-900 flex flex-col",
                
                // Menu bar
                MenuBar {}
                
                // Main application layout
                div { class: "flex-1 flex overflow-hidden",
                    
                    // Sidebar with file tree
                    Sidebar { 
                        collapsed: false,
                        width: 250,
                        DesktopFileTree { 
                            onfile_select: move |file_path| {
                                info!("Selected file: {:?}", file_path);
                                // Handle file selection
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
                        DesktopAIChatInterface {
                            model_manager: None, // TODO: Fix type compatibility with Resource
                        }
                    }
                }
                
                // Status bar
                StatusBar {}
            }
        }
    }
}

#[component]
fn MenuBar() -> Element {
    rsx! {
        div { class: "h-8 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center px-4",
            div { class: "text-sm text-gray-600 dark:text-gray-300",
                "File | Edit | View | Tools | Help"
            }
            div { class: "flex-1" }
            div { class: "text-sm text-gray-600 dark:text-gray-300",
                "LaTeX IDE v0.1.0"
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
            
            // Editor using shared component
            div { class: "flex-1",
                if current_document.read().is_some() {
                    DesktopTextEditor {
                        initial_content: Some(document_content.read().clone()),
                        onchange: None, // TODO: Fix event handler compatibility
                        show_line_numbers: Some(true),
                        syntax_highlighting: Some(true),
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
fn StatusBar() -> Element {
    let mut theme = use_theme();
    
    rsx! {
        div { class: "h-6 bg-blue-600 text-white text-xs flex items-center justify-between px-4",
            div { class: "flex items-center space-x-4",
                span { "Ready" }
                span { "UTF-8" }
                span { "LaTeX" }
                span { "Line 1, Col 1" }
            }
            
            div { class: "flex items-center space-x-4",
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
                
                span { "Connected" }
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