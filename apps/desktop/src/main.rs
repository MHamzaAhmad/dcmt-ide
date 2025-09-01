use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_yrs_collab::{CollaborationEngine, LaTeXDocument, UserInfo};
use latex_ide_model_manager::ModelManager;
use uuid::Uuid;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

mod components;
mod hooks;
mod state;

use components::*;
use state::AppState;

const WINDOW_TITLE: &str = "LaTeX IDE - Desktop";

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting LaTeX IDE Desktop application");

    // Launch Dioxus desktop app
    LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new()
            .with_window_title(WINDOW_TITLE)
            .with_window_size((1400, 900))
            .with_min_window_size((800, 600))
            .with_icon(include_bytes!("../assets/icon.ico"))
            .with_menu(create_menu())
        )
        .launch(App);

    Ok(())
}

#[component]
fn App() -> Element {
    // Initialize application state
    let app_state = use_context_provider(|| AppState::new());
    
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
                        FileTree { 
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
                                PreviewPane {}
                            }
                        }
                    }
                    
                    // AI Chat sidebar  
                    div { class: "w-80 border-l border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800",
                        AIChatInterface {
                            model_manager: model_manager,
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
fn FileTree(onfile_select: EventHandler<String>) -> Element {
    let current_dir = use_signal(|| std::env::current_dir().unwrap_or_default());
    
    rsx! {
        div { class: "h-full overflow-auto p-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                "Files"
            }
            
            // Mock file tree for now
            div { class: "space-y-1",
                FileItem { 
                    name: "document.tex",
                    is_file: true,
                    onclick: move |_| onfile_select.call("document.tex".to_string())
                }
                FileItem { 
                    name: "figures/",
                    is_file: false,
                    onclick: move |_| {}
                }
                FileItem { 
                    name: "references.bib",
                    is_file: true,
                    onclick: move |_| onfile_select.call("references.bib".to_string())
                }
            }
        }
    }
}

#[component]
fn FileItem(name: String, is_file: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let icon = if is_file { "📄" } else { "📁" };
    
    rsx! {
        div { 
            class: "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer",
            onclick: move |evt| onclick.call(evt),
            
            span { "{icon}" }
            span { "{name}" }
        }
    }
}

#[component]
fn EditorPane(collab_engine: Resource<Arc<CollaborationEngine>>) -> Element {
    let current_document = use_signal(|| None::<LaTeXDocument>);
    let document_content = use_signal(|| String::new());
    
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
            
            // Editor
            div { class: "flex-1",
                if let Some(_doc) = current_document.read().as_ref() {
                    latex_ide_editor::TextEditor {
                        initial_content: Some(document_content.read().clone()),
                        onchange: Some(move |content| {
                            document_content.set(content);
                        }),
                        show_line_numbers: true,
                        syntax_highlighting: true,
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
fn PreviewPane() -> Element {
    rsx! {
        div { class: "h-full bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700",
            
            // PDF controls
            div { class: "h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4",
                div { class: "flex items-center space-x-2",
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Compile document
                        },
                        "Compile"
                    }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Refresh PDF
                        },
                        "🔄"
                    }
                }
                
                div { class: "flex items-center space-x-2 text-sm text-gray-600 dark:text-gray-400",
                    span { "Page 1 of 1" }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Zoom out
                        },
                        "−"
                    }
                    
                    span { "100%" }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Zoom in
                        },
                        "+"
                    }
                }
            }
            
            // PDF viewer
            div { class: "flex-1 overflow-auto p-4",
                div { class: "bg-white shadow-lg mx-auto",
                    style: "width: 210mm; min-height: 297mm;",
                    
                    div { class: "h-full flex items-center justify-center text-gray-500",
                        "PDF Preview\n(Compile document to see output)"
                    }
                }
            }
        }
    }
}

#[component]
fn AIChatInterface(model_manager: Resource<Option<Arc<ModelManager>>>) -> Element {
    let selected_model = use_signal(|| "llama-3.1".to_string());
    let chat_messages = use_signal(|| Vec::<String>::new());
    let current_message = use_signal(|| String::new());
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // AI Chat header
            div { class: "p-4 border-b border-gray-200 dark:border-gray-700",
                h3 { class: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-2",
                    "AI Assistant"
                }
                
                // Model selection
                Dropdown {
                    items: vec![
                        DropdownItem { 
                            id: "llama-3.1".to_string(), 
                            label: "Llama 3.1 (Local)".to_string(),
                            icon: None 
                        },
                        DropdownItem { 
                            id: "gpt-4o".to_string(), 
                            label: "GPT-4o (OpenAI)".to_string(),
                            icon: None 
                        },
                        DropdownItem { 
                            id: "claude-sonnet".to_string(), 
                            label: "Claude 3.5 Sonnet".to_string(),
                            icon: None 
                        }
                    ],
                    selected: Some(selected_model.read().clone()),
                    onselect: move |model_id| {
                        selected_model.set(model_id);
                    },
                    placeholder: "Select Model".to_string(),
                }
            }
            
            // Chat messages
            div { class: "flex-1 overflow-auto p-4 space-y-4",
                if chat_messages.read().is_empty() {
                    div { class: "text-center text-gray-500 mt-8",
                        "Start a conversation with your AI assistant.\nAsk about LaTeX syntax, document structure, or get help with your writing."
                    }
                } else {
                    for message in chat_messages.read().iter() {
                        div { class: "bg-gray-100 dark:bg-gray-700 rounded-lg p-3",
                            "{message}"
                        }
                    }
                }
            }
            
            // Message input
            div { class: "p-4 border-t border-gray-200 dark:border-gray-700",
                div { class: "flex space-x-2",
                    Input {
                        value: Some(current_message.read().clone()),
                        placeholder: Some("Ask about LaTeX, document structure, or get writing help...".to_string()),
                        onchange: move |value| {
                            current_message.set(value);
                        }
                    }
                    
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {
                            let message = current_message.read().clone();
                            if !message.trim().is_empty() {
                                chat_messages.write().push(format!("You: {}", message));
                                chat_messages.write().push(format!("AI: I can help you with that LaTeX question!"));
                                current_message.set(String::new());
                            }
                        },
                        "Send"
                    }
                }
            }
        }
    }
}

#[component]
fn StatusBar() -> Element {
    let theme = use_theme();
    
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

fn create_menu() -> dioxus::desktop::tao::menu::MenuBar {
    use dioxus::desktop::tao::menu::*;
    
    MenuBar::new()
        .add_submenu(Submenu::new("File", true)
            .add_item(MenuItem::new("New", true, None))
            .add_item(MenuItem::new("Open", true, None))
            .add_item(MenuItem::new("Save", true, None))
            .add_separator()
            .add_item(MenuItem::new("Exit", true, None)))
        .add_submenu(Submenu::new("Edit", true)
            .add_item(MenuItem::new("Undo", true, None))
            .add_item(MenuItem::new("Redo", true, None))
            .add_separator()
            .add_item(MenuItem::new("Cut", true, None))
            .add_item(MenuItem::new("Copy", true, None))
            .add_item(MenuItem::new("Paste", true, None)))
        .add_submenu(Submenu::new("View", true)
            .add_item(MenuItem::new("Toggle Theme", true, None))
            .add_item(MenuItem::new("Toggle Sidebar", true, None)))
}