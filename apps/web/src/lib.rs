use dioxus::prelude::*;
use latex_ide_ui::*;
use dioxus_signals::{Signal, Readable};
use dioxus_hooks::{use_signal, use_effect};
use wasm_bindgen::prelude::*;

// Import components from shared crates
use latex_ide_editor::web::WebEditorPane;
use latex_ide_chat::web::BrowserCapabilities;
use latex_ide_file_manager::{web::{WebFileTree, WebGitClient, GitStatusResponse, WebGitPanel}, ProjectManager};
use latex_ide_pdf_viewer::web::WebPreviewPane;
use latex_ide_ui::web::{AppHeader, hooks::use_browser_capabilities, GitStatus, SidebarView};

mod transport;

// Create a simple function to convert between BrowserCapabilities types
#[allow(dead_code)]
fn convert_capabilities(local_caps: &latex_ide_ui::web::hooks::BrowserCapabilities) -> BrowserCapabilities {
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
    let mut document_content = use_signal(|| String::new());
    let mut current_file_path = use_signal(|| None::<String>);
    let mut project_manager = use_signal(|| ProjectManager::new());
    let local_capabilities = use_browser_capabilities();
    let _browser_capabilities = use_signal(|| convert_capabilities(&local_capabilities.read()));
    
    // Sidebar state
    let mut sidebar_view = use_signal(|| SidebarView::Explorer);
    
    // Git integration
    let git_client = use_signal(|| WebGitClient::new());
    let git_status = use_signal(|| None::<GitStatusResponse>);
    
    // Initialize workspace project in background
    use_effect(move || {
        let mut pm = project_manager.write();
        match pm.open_workspace(std::path::PathBuf::from(".")) {
            Ok(_) => {
                tracing::info!("Opened workspace project successfully");
            }
            Err(e) => {
                tracing::warn!("Could not open workspace: {}. File tree will still work via WebTransport.", e);
            }
        }
    });
    
    // Initialize Git integration
    use_effect(move || {
        let git_client = git_client.clone();
        let mut git_status = git_status.clone();
        
        wasm_bindgen_futures::spawn_local(async move {
            match git_client.read().init_repository(".".to_string()).await {
                Ok(status) => {
                    git_status.set(Some(status));
                    tracing::info!("Git integration initialized successfully");
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize Git integration: {}", e);
                }
            }
        });
    });
    
    rsx! {
        ThemeProvider {
            div {
                id: "app",
                class: "h-screen w-screen bg-white dark:bg-zinc-950 flex flex-col",
                
                // Enhanced header with sidebar controls
                AppHeader {
                    sidebar_view: sidebar_view,
                    git_status: use_signal(move || git_status.read().as_ref().map(convert_git_status)),
                }
                
                div { class: "flex-1 flex overflow-hidden",
                    // Left sidebar - Conditional content based on sidebar_view
                    if *sidebar_view.read() != SidebarView::None {
                        div { class: "w-64 border-r border-zinc-200 dark:border-zinc-800 transition-all duration-200",
                            match *sidebar_view.read() {
                                SidebarView::Explorer => rsx! {
                                    WebFileTree { 
                                        project_manager: project_manager,
                                        onfile_select: move |file_path: String| {
                                            tracing::info!("Loading file: {}", file_path);
                                            current_file_path.set(Some(file_path.clone()));
                                            
                                            // Load file content via WebTransport FileOp::Download
                                            #[cfg(target_arch = "wasm32")]
                                            wasm_bindgen_futures::spawn_local(async move {
                                                use latex_ide_file_manager::web::transport::download_file;
                                                
                                                match download_file(&file_path).await {
                                                    Ok(content_bytes) => {
                                                        let content = String::from_utf8_lossy(&content_bytes).to_string();
                                                        document_content.set(content);
                                                        tracing::info!("Received file content for: {}", file_path);
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to download file {}: {}", file_path, e);
                                                        document_content.set(format!("// Error loading file: {}", e));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                },
                                SidebarView::Git => rsx! {
                                    WebGitPanel {
                                        git_client: git_client,
                                        git_status: git_status,
                                    }
                                },
                                SidebarView::None => rsx! { div {} }
                            }
                        }
                    }
                    
                    // Main content area
                    div { class: "flex-1 flex",
                        // Editor pane
                        div { class: "flex-1 border-r border-zinc-200 dark:border-zinc-800",
                            WebEditorPane { 
                                document_content: document_content,
                                current_file: current_file_path
                            }
                        }
                        
                        // Preview pane
                        div { class: "flex-1",
                            WebPreviewPane { 
                                document_content: document_content,
                                current_file: Some(current_file_path)
                            }
                        }
                    }
                }
            }
        }
    }
}

// Convert GitStatusResponse to GitStatus for AppHeader
fn convert_git_status(status: &GitStatusResponse) -> GitStatus {
    GitStatus {
        current_branch: status.current_branch.clone(),
        session_branch: status.session_branch.clone(),
        has_changes: status.has_changes,
    }
}
