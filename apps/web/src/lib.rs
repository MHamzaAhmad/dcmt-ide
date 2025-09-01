use dioxus::prelude::*;
use latex_ide_ui::*;
use dioxus_signals::{Signal, Readable};
use dioxus_hooks::use_signal;
use wasm_bindgen::prelude::*;

// Import components from shared crates
use latex_ide_editor::web::WebTextEditor;
use latex_ide_chat::web::{WebAIChatInterface, BrowserCapabilities};
use latex_ide_file_manager::{web::WebFileTree, ProjectManager};
use latex_ide_pdf_viewer::web::WebPreviewPane;

mod hooks;
mod transport;

// Create a simple function to convert between BrowserCapabilities types
fn convert_capabilities(local_caps: &hooks::BrowserCapabilities) -> BrowserCapabilities {
    BrowserCapabilities {
        server_sent_events: local_caps.server_sent_events,
        websocket_supported: local_caps.websocket_supported,
        webtransport_supported: local_caps.webtransport_supported,
    }
}

// Entry point for manual initialization (called from JavaScript)
#[wasm_bindgen]
pub fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Tracing is initialized by Dioxus
    tracing::info!("Starting LaTeX IDE Web application");
    
    // Launch Dioxus web app 
    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch::launch_cfg(App, dioxus_web::Config::new());
}

// Alternative entry point for dx serve
#[cfg(feature = "dev")]
pub fn run() {
    main();
}

#[component]
fn App() -> Element {
    let document_content = use_signal(|| "% LaTeX document\n\\documentclass{article}\n\\begin{document}\nHello World!\n\\end{document}".to_string());
    let project_manager = use_signal(|| ProjectManager::new());
    let local_capabilities = hooks::use_browser_capabilities();
    let browser_capabilities = use_signal(|| convert_capabilities(&local_capabilities.read()));
    
    rsx! {
        ThemeProvider {
            div {
                id: "app",
                class: "h-screen w-screen bg-white dark:bg-gray-900 flex flex-col p-4",
                
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-4",
                    "LaTeX IDE Web"
                }
                
                div { class: "flex-1 flex gap-4",
                    // Left sidebar - File Tree
                    div { class: "w-64 border-r border-gray-200 dark:border-gray-700",
                        WebFileTree { project_manager: project_manager }
                    }
                    
                    // Main content area
                    div { class: "flex-1 flex gap-4",
                        // Editor pane
                        div { class: "flex-1",
                            EditorPane { document_content: document_content }
                        }
                        
                        // Preview pane
                        div { class: "flex-1",
                            WebPreviewPane { document_content: document_content }
                        }
                    }
                    
                    // Right sidebar - AI Chat
                    div { class: "w-80 border-l border-gray-200 dark:border-gray-700",
                        WebAIChatInterface { capabilities: browser_capabilities }
                    }
                }
                
                // Status bar
                StatusBar { capabilities: browser_capabilities }
            }
        }
    }
}

#[component]
fn EditorPane(document_content: Signal<String>) -> Element {
    rsx! {
        div { class: "h-full flex flex-col",
            
            // Document tabs
            div { class: "border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800",
                Tabs {
                    tabs: vec![
                        Tab {
                            id: "document".to_string(),
                            label: "document.tex".to_string(),
                            content: rsx! { div {} }
                        }
                    ],
                    active_tab: Some("document".to_string()),
                    onchange: move |_| {}
                }
            }
            
            // Editor using the new WebTextEditor component
            div { class: "flex-1",
                WebTextEditor {
                    content: document_content,
                }
            }
        }
    }
}

#[component]
fn StatusBar(capabilities: Signal<BrowserCapabilities>) -> Element {
    let caps = capabilities.read();
    
    rsx! {
        div { class: "h-6 bg-blue-600 text-white text-xs flex items-center justify-between px-4",
            div { class: "flex items-center space-x-4",
                span { "Ready" }
                span { "WebAssembly" }
                span { "UTF-8" }
                span { "LaTeX" }
            }
            
            div { class: "flex items-center space-x-4",
                span {
                    class: if caps.webtransport_supported { "text-green-300" } else { "text-yellow-300" },
                    if caps.webtransport_supported { "WebTransport" } else { "WebSocket" }
                }
                
                span { "ywasm CRDT" }
            }
        }
    }
}