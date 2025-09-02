use dioxus::prelude::*;
use latex_ide_ui::*;
use dioxus_signals::{Signal, Readable};
use dioxus_hooks::{use_signal, use_effect};
use wasm_bindgen::prelude::*;

// Import components from shared crates
use latex_ide_editor::web::WebCodeMirrorEditor;
use latex_ide_chat::web::{WebAIChatInterface, BrowserCapabilities};
use latex_ide_file_manager::{web::WebFileTree, ProjectManager};
use latex_ide_pdf_viewer::web::WebPreviewPane;

mod hooks;
mod transport;

// Create a simple function to convert between BrowserCapabilities types
#[allow(dead_code)]
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
    let mut document_content = use_signal(|| String::new());
    let mut current_file_path = use_signal(|| None::<String>);
    let mut project_manager = use_signal(|| ProjectManager::new());
    let local_capabilities = hooks::use_browser_capabilities();
    let browser_capabilities = use_signal(|| convert_capabilities(&local_capabilities.read()));
    
    // Initialize workspace project and load main file
    use_effect(move || {
        let mut pm = project_manager.write();
        match pm.open_workspace(std::path::PathBuf::from("/workspace")) {
            Ok(_) => {
                tracing::info!("Opened workspace project");
                
                // Auto-load main.tex file if it exists
                if let Some(project) = pm.get_current_project() {
                    if let Some(main_file) = &project.main_file {
                        let main_file_path = main_file.display().to_string();
                        tracing::info!("Auto-loading main file: {}", main_file_path);
                        current_file_path.set(Some(main_file_path.clone()));
                        
                        // Load the main file content via WebTransport
                        let _content_signal = document_content.clone();
                        #[cfg(target_arch = "wasm32")]
                        wasm_bindgen_futures::spawn_local(async move {
                            use latex_ide_file_manager::web::transport::download_file;
                            
                            let mut content_signal_clone = document_content.clone();
                            
                            match download_file("main.tex").await {
                                Ok(content_bytes) => {
                                    let content = String::from_utf8_lossy(&content_bytes).to_string();
                                    content_signal_clone.set(content);
                                    tracing::info!("Auto-loaded main file content");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to auto-load main file: {}", e);
                                    content_signal_clone.set(format!("// Error auto-loading main file: {}", e));
                                }
                            }
                        });
                    }
                } else {
                    // Fallback: try to auto-load main.tex anyway
                    tracing::info!("No project main file configured, attempting to load main.tex");
                    current_file_path.set(Some("main.tex".to_string()));
                    
                    #[cfg(target_arch = "wasm32")]
                    wasm_bindgen_futures::spawn_local(async move {
                        use latex_ide_file_manager::web::transport::download_file;
                        
                        let mut content_signal_clone = document_content.clone();
                        
                        match download_file("main.tex").await {
                            Ok(content_bytes) => {
                                let content = String::from_utf8_lossy(&content_bytes).to_string();
                                content_signal_clone.set(content);
                                tracing::info!("Auto-loaded fallback main.tex");
                            }
                            Err(e) => {
                                tracing::error!("Failed to auto-load fallback main.tex: {}", e);
                                content_signal_clone.set(format!("// Error loading main.tex: {}", e));
                            }
                        }
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to open workspace: {}", e);
            }
        }
    });
    
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
                        WebFileTree { 
                            project_manager: project_manager,
                            onfile_select: move |file_path: String| {
                                tracing::info!("Loading file: {}", file_path);
                                current_file_path.set(Some(file_path.clone()));
                                
                                // Load file content via WebTransport FileOp::Download
                                let mut content_signal_clone = document_content.clone();
                                #[cfg(target_arch = "wasm32")]
                                wasm_bindgen_futures::spawn_local(async move {
                                    use latex_ide_file_manager::web::transport::download_file;
                                    
                                    match download_file(&file_path).await {
                                        Ok(content_bytes) => {
                                            let content = String::from_utf8_lossy(&content_bytes).to_string();
                                            content_signal_clone.set(content);
                                            tracing::info!("Received file content for: {}", file_path);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to download file {}: {}", file_path, e);
                                            content_signal_clone.set(format!("// Error loading file: {}", e));
                                        }
                                    }
                                });
                            }
                        }
                    }
                    
                    // Main content area
                    div { class: "flex-1 flex gap-4",
                        // Editor pane
                        div { class: "flex-1",
                            EditorPane { 
                                document_content: document_content,
                                current_file: current_file_path
                            }
                        }
                        
                        // Preview pane
                        div { class: "flex-1",
                            WebPreviewPane { 
                                document_content: document_content,
                                current_file: current_file_path
                            }
                        }
                    }
                    
                    // Right sidebar - AI Chat
                    div { class: "w-80 border-l border-gray-200 dark:border-gray-700",
                        WebAIChatInterface { capabilities: browser_capabilities }
                    }
                }
                
                // Status bar
                StatusBar { 
                    capabilities: browser_capabilities,
                    project_manager: project_manager,
                    current_file: current_file_path
                }
            }
        }
    }
}

#[component]
fn EditorPane(
    document_content: Signal<String>,
    current_file: Signal<Option<String>>
) -> Element {
    let current_file_guard = current_file.read();
    let file_name = current_file_guard.as_ref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("No file selected");
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // Document tabs
            div { class: "border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800",
                if current_file.read().is_some() {
                    Tabs {
                        tabs: vec![
                            Tab {
                                id: "current".to_string(),
                                label: file_name.to_string(),
                                content: rsx! { div {} }
                            }
                        ],
                        active_tab: Some("current".to_string()),
                        onchange: move |_| {}
                    }
                } else {
                    div { class: "px-4 py-2 text-sm text-gray-500",
                        "Select a file from the workspace to start editing"
                    }
                }
            }
            
            // Editor using CodeMirror
            div { class: "flex-1",
                if current_file.read().is_some() {
                    WebCodeMirrorEditor {
                        content: document_content,
                        enable_ai_suggestions: true,
                        enable_pdf_sync: true,
                        on_change: Some(EventHandler::new(move |new_content: String| {
                            // Update the document content signal
                            document_content.set(new_content.clone());
                            
                            // Auto-save to server when content changes and trigger compilation
                            if let Some(file_path) = current_file.read().as_ref() {
                                let file_path_clone = file_path.clone();
                                #[cfg(target_arch = "wasm32")]
                                wasm_bindgen_futures::spawn_local(async move {
                                    use latex_ide_file_manager::web::transport::{upload_file, FileTransportClient, TransportMessage as FileTransportMessage, FileOp};
                                    
                                    // First, save the file
                                    match upload_file(&file_path_clone, new_content.clone().into_bytes()).await {
                                        Ok(_) => {
                                            tracing::info!("Auto-saved file: {}", file_path_clone);
                                            
                                            // Then trigger compilation if it's a .tex file
                                            if file_path_clone.ends_with(".tex") {
                                                let mut client = FileTransportClient::new();
                                                match client.connect("ws://localhost:3001").await {
                                                    Ok(_) => {
                                                        let document_id = file_path_clone.replace(".tex", "");
                                                        let compile_msg = FileTransportMessage::CompilationRequest {
                                                            document_id,
                                                            content: new_content,
                                                            engine: "pdflatex".to_string(),
                                                        };
                                                        
                                                        match client.send_operation(FileOp::Upload { 
                                                            name: format!("compile:{}", file_path_clone), 
                                                            content: serde_json::to_vec(&compile_msg).unwrap_or_default()
                                                        }).await {
                                                            Ok(FileTransportMessage::CompilationResult { success, log, .. }) => {
                                                                if success {
                                                                    tracing::info!("Compilation successful for: {}", file_path_clone);
                                                                } else {
                                                                    tracing::error!("Compilation failed for {}: {}", file_path_clone, log);
                                                                }
                                                            }
                                                            Ok(_) => {
                                                                tracing::info!("Compilation request sent for: {}", file_path_clone);
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("Compilation request failed: {}", e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to connect for compilation: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to auto-save file {}: {}", file_path_clone, e);
                                        }
                                    }
                                });
                            }
                        })),
                    }
                } else {
                    div { class: "h-full flex items-center justify-center text-gray-500",
                        div { class: "text-center",
                            div { class: "text-6xl mb-4", "📝" }
                            div { class: "text-lg mb-2", "No File Selected" }
                            div { class: "text-sm", "Choose a file from the workspace to start editing" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusBar(
    capabilities: Signal<BrowserCapabilities>, 
    project_manager: Signal<ProjectManager>,
    current_file: Signal<Option<String>>
) -> Element {
    let caps = capabilities.read();
    
    rsx! {
        div { class: "h-6 bg-blue-600 text-white text-xs flex items-center justify-between px-4",
            div { class: "flex items-center space-x-4",
                span { "Ready" }
                
                // Workspace info
                span { 
                    if let Some(project) = project_manager.read().get_current_project() {
                        "📁 {project.name}"
                    } else {
                        "📁 /workspace"
                    }
                }
                
                // Current file info
                if let Some(file_path) = current_file.read().as_ref() {
                    span { 
                        if let Some(file_name) = std::path::Path::new(file_path).file_name().and_then(|n| n.to_str()) {
                            "📝 {file_name}"
                        } else {
                            "📝 File"
                        }
                    }
                }
                
                // Main file info
                if let Some(main_file) = project_manager.read().get_current_project()
                    .and_then(|p| p.main_file.as_ref()) {
                    span { "🎯 {main_file.display()}" }
                }
                
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