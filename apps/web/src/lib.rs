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
    let document_content = use_signal(|| String::new());
    let mut current_file_path = use_signal(|| None::<String>);
    let mut project_manager = use_signal(|| ProjectManager::new());
    let local_capabilities = hooks::use_browser_capabilities();
    let browser_capabilities = use_signal(|| convert_capabilities(&local_capabilities.read()));
    
    // Sidebar visibility states
    let show_file_explorer = use_signal(|| true);
    let show_ai_chat = use_signal(|| false);
    
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
                class: "h-screen w-screen bg-white dark:bg-zinc-950 flex flex-col",
                
                // Enhanced header with sidebar controls
                AppHeader {
                    show_file_explorer: show_file_explorer,
                    show_ai_chat: show_ai_chat,
                }
                
                div { class: "flex-1 flex overflow-hidden",
                    // Left sidebar - File Tree (collapsible)
                    if *show_file_explorer.read() {
                        div { class: "w-64 border-r border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 transition-all duration-300",
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
                    }
                    
                    // Main content area
                    div { class: "flex-1 flex",
                        // Editor pane
                        div { class: "flex-1 border-r border-zinc-200 dark:border-zinc-800",
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
                    
                    // Right sidebar - AI Chat (collapsible)
                    if *show_ai_chat.read() {
                        div { class: "w-80 border-l border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 transition-all duration-300",
                            WebAIChatInterface { capabilities: browser_capabilities }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AppHeader(
    mut show_file_explorer: Signal<bool>,
    mut show_ai_chat: Signal<bool>,
) -> Element {
    let mut theme = use_theme();
    
    rsx! {
        div { 
            class: "h-14 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex items-center justify-between px-4",
            
            // Left side - Panel toggles
            div { class: "flex items-center space-x-2",
                // File Explorer toggle
                button {
                    class: if *show_file_explorer.read() {
                        "p-2 rounded-lg bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 transition-colors"
                    } else {
                        "p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-400 transition-colors"
                    },
                    onclick: move |_| {
                        let current = *show_file_explorer.read();
                        show_file_explorer.set(!current);
                    },
                    title: "Toggle File Explorer",
                    
                    // Folder icon
                    svg {
                        class: "w-5 h-5",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
                    }
                }
                
                // Search panel toggle (placeholder for future)
                button {
                    class: "p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-400 transition-colors",
                    title: "Search (Coming Soon)",
                    disabled: true,
                    
                    // Search icon
                    svg {
                        class: "w-5 h-5",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        circle { cx: "11", cy: "11", r: "8" }
                        path { d: "m21 21-4.35-4.35" }
                    }
                }
                
                // Git panel toggle (placeholder for future)
                button {
                    class: "p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-400 transition-colors",
                    title: "Version Control (Coming Soon)",
                    disabled: true,
                    
                    // Git branch icon
                    svg {
                        class: "w-5 h-5",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        line { x1: "6", y1: "3", x2: "6", y2: "15" }
                        circle { cx: "18", cy: "6", r: "3" }
                        circle { cx: "6", cy: "18", r: "3" }
                        path { d: "M18 9a9 9 0 0 1-9 9" }
                    }
                }
                
                // Separator
                div { class: "w-px h-6 bg-zinc-300 dark:bg-zinc-700 mx-2" }
                
                // Project name / title
                h1 { class: "text-sm font-medium text-zinc-700 dark:text-zinc-300",
                    "LaTeX Editor"
                }
            }
            
            // Right side - AI Chat and Theme toggle
            div { class: "flex items-center space-x-2",
                // AI Chat toggle
                button {
                    class: if *show_ai_chat.read() {
                        "p-2 rounded-lg bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 transition-colors"
                    } else {
                        "p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-400 transition-colors"
                    },
                    onclick: move |_| {
                        let current = *show_ai_chat.read();
                        show_ai_chat.set(!current);
                    },
                    title: "Toggle AI Assistant",
                    
                    // AI/Chat icon
                    svg {
                        class: "w-5 h-5",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" }
                        line { x1: "9", y1: "10", x2: "9", y2: "10" }
                        line { x1: "15", y1: "10", x2: "15", y2: "10" }
                    }
                }
                
                // Separator
                div { class: "w-px h-6 bg-zinc-300 dark:bg-zinc-700 mx-2" }
                
                // Theme toggle button
                button {
                    class: "p-2 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors",
                    onclick: move |_| {
                        let new_theme = theme.read().toggle();
                        theme.set(new_theme);
                    },
                    title: if *theme.read() == Theme::Light { "Switch to Dark Mode" } else { "Switch to Light Mode" },
                    
                    if *theme.read() == Theme::Light {
                        // Moon icon for dark mode
                        svg {
                            class: "w-5 h-5 text-zinc-600 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path {
                                d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
                            }
                        }
                    } else {
                        // Sun icon for light mode
                        svg {
                            class: "w-5 h-5 text-zinc-600 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            circle { cx: "12", cy: "12", r: "5" }
                            line { x1: "12", y1: "1", x2: "12", y2: "3" }
                            line { x1: "12", y1: "21", x2: "12", y2: "23" }
                            line { x1: "4.22", y1: "4.22", x2: "5.64", y2: "5.64" }
                            line { x1: "18.36", y1: "18.36", x2: "19.78", y2: "19.78" }
                            line { x1: "1", y1: "12", x2: "3", y2: "12" }
                            line { x1: "21", y1: "12", x2: "23", y2: "12" }
                            line { x1: "4.22", y1: "19.78", x2: "5.64", y2: "18.36" }
                            line { x1: "18.36", y1: "5.64", x2: "19.78", y2: "4.22" }
                        }
                    }
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
        div { class: "h-full flex flex-col bg-white dark:bg-zinc-950",
            
            // Document tabs - minimal style
            div { class: "h-10 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900",
                if current_file.read().is_some() {
                    div { class: "h-full flex items-center px-4",
                        div { class: "px-3 py-1 text-sm font-medium text-zinc-900 dark:text-zinc-100 border-b-2 border-zinc-900 dark:border-zinc-100",
                            "{file_name}"
                        }
                    }
                } else {
                    div { class: "h-full flex items-center px-4 text-sm text-zinc-500 dark:text-zinc-400",
                        "No file selected"
                    }
                }
            }
            
            // Editor using CodeMirror
            div { class: "flex-1 overflow-hidden",
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
                    div { class: "h-full flex items-center justify-center",
                        div { class: "text-center",
                            svg {
                                class: "w-16 h-16 mx-auto mb-4 text-zinc-300 dark:text-zinc-700",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                view_box: "0 0 24 24",
                                path {
                                    d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                }
                                polyline {
                                    points: "14 2 14 8 20 8"
                                }
                                line { x1: "16", y1: "13", x2: "8", y2: "13" }
                                line { x1: "16", y1: "17", x2: "8", y2: "17" }
                                polyline {
                                    points: "10 9 9 9 8 9"
                                }
                            }
                            div { class: "text-lg font-medium mb-2 text-zinc-900 dark:text-zinc-100", "No File Selected" }
                            div { class: "text-sm text-zinc-500 dark:text-zinc-400", "Choose a file from the workspace to start editing" }
                        }
                    }
                }
            }
        }
    }
}

