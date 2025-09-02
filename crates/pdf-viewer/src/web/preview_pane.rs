use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};
use crate::controls::PDFControls;
#[cfg(feature = "native-git")]
use latex_ide_git_manager::{SessionManager, GitRepository, HistoryViewer};
use chrono::{DateTime, Utc};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    gloo_timers,
    js_sys,
    web_sys,
};

// Fallback spawn_local for non-wasm32 targets
#[cfg(not(target_arch = "wasm32"))]
fn spawn_local<F>(_future: F) 
where
    F: std::future::Future<Output = ()> + 'static,
{
    tracing::info!("spawn_local called on non-wasm32 target - no-op");
}

#[derive(Clone, Debug)]
pub enum CompilationStatus {
    Ready,
    Compiling,
    Success,
    Error,
}

/// Transport message placeholder - in real implementation this would come from transport crate
#[derive(Clone, Debug)]
pub struct CompilationRequest {
    pub document_id: String,
    pub content: String,
    pub engine: String,
}

#[component]
pub fn WebPreviewPane(
    document_content: Signal<String>,
    current_file: Option<Signal<Option<String>>>,
    #[props(default)] refresh_trigger: Option<Signal<u32>>,
    #[props(default)] session_manager: Option<Signal<Option<String>>>, // Just store a placeholder string for WASM
) -> Element {
    let mut compilation_status = use_signal(|| CompilationStatus::Ready);
    let pdf_url = use_signal(|| None::<String>);
    let mut auto_compiled = use_signal(|| false);
    let available_versions = use_signal(|| Vec::<String>::new());
    let current_version = use_signal(|| None::<String>);
    
    let has_content = !document_content.read().is_empty();
    let current_file_name = current_file
        .and_then(|cf| cf.read().as_ref().and_then(|path| 
            std::path::Path::new(path).file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
        ));
    
    // Clone the file name for use in closures
    let current_file_name_for_closures = current_file_name.clone();
    let current_file_name_for_trigger = current_file_name.clone();
    
    // Load available versions when session manager is ready
    use_effect(move || {
        if session_manager.is_some() {
            let mut versions = available_versions.clone();
            let mut current_ver = current_version.clone();
            
            spawn_local(async move {
                #[cfg(target_arch = "wasm32")]
                {
                    use latex_ide_git_transport::web::WebGitTransport;
                    
                    let git_transport = WebGitTransport::new();
                    match git_transport.get_all_versions().await {
                        Ok(version_list) => {
                            versions.set(version_list.clone());
                            // Set current version to latest if available
                            if let Some(latest) = version_list.last() {
                                current_ver.set(Some(latest.clone()));
                            }
                            tracing::info!("Loaded {} available versions", version_list.len());
                        }
                        Err(e) => {
                            tracing::error!("Failed to load versions: {}", e);
                        }
                    }
                }
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    tracing::info!("Version loading not implemented for non-wasm32 target");
                }
            });
        }
    });
    
    // Auto-compile on startup when content is available
    use_effect(move || {
        let has_content = !document_content.read().is_empty();
        if has_content && !*auto_compiled.read() {
            auto_compiled.set(true);
            tracing::info!("Auto-compiling PDF on startup");
            
            // Trigger compilation
            let status = compilation_status.clone();
            let pdf = pdf_url.clone();
            let content = document_content.read().clone();
            let file_name = current_file_name_for_closures.clone().unwrap_or_else(|| "document".to_string());
            
            compile_latex(content, file_name, status, pdf, available_versions.clone(), current_version.clone());
        }
    });
    
    // Watch for refresh trigger changes (e.g., branch switching)
    if let Some(trigger) = refresh_trigger {
        use_effect(move || {
            let trigger_value = *trigger.read();
            if trigger_value > 0 && !document_content.read().is_empty() {
                tracing::info!("PDF refresh triggered ({})", trigger_value);
                
                let status = compilation_status.clone();
                let pdf = pdf_url.clone();
                let content = document_content.read().clone();
                let file_name = current_file_name_for_trigger.clone().unwrap_or_else(|| "document".to_string());
                
                compile_latex(content, file_name, status, pdf, available_versions.clone(), current_version.clone());
            }
        });
    }
    let current_file_name_for_closure = current_file_name.clone();
    
    rsx! {
        div { class: "h-full bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 flex flex-col",
            
            // Session History Display (above controls)
            SessionHistoryBar {
                session_manager: session_manager,
            }
            
            // PDF controls with version management
            div { class: "border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800",
                div { class: "flex items-center justify-between px-4 py-2",
                    div { class: "flex items-center space-x-2",
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Small,
                            disabled: matches!(*compilation_status.read(), CompilationStatus::Compiling) || !has_content,
                            onclick: move |_| {
                                if !has_content {
                                    return;
                                }
                                
                                compilation_status.set(CompilationStatus::Compiling);
                                
                                let _content = document_content.read().clone();
                                let status = compilation_status.clone();
                                let pdf = pdf_url.clone();
                                let file_name = current_file_name_for_closure.clone().unwrap_or_else(|| "document".to_string());
                                
                                compile_latex(_content, file_name, status, pdf, available_versions.clone(), current_version.clone());
                            },
                            match *compilation_status.read() {
                                CompilationStatus::Compiling => "⏳ Compiling...",
                                CompilationStatus::Success => "✅ Compiled",
                                CompilationStatus::Error => "❌ Error",
                                CompilationStatus::Ready => if has_content { "▶️ Compile" } else { "📄 No Content" }
                            }
                        }
                        
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Small,
                            onclick: move |_| {
                                // Download PDF
                                tracing::info!("Download PDF clicked");
                            },
                            "📥"
                        }
                        
                        // Version controls
                        if !available_versions.read().is_empty() {
                            div {
                                class: "flex items-center gap-2 ml-4 border-l border-gray-200 dark:border-gray-700 pl-4",
                                span {
                                    class: "text-sm text-gray-600 dark:text-gray-400",
                                    "Version:"
                                }
                                
                                if let Some(version) = current_version.read().as_ref() {
                                    span {
                                        class: "text-sm font-mono text-blue-600 dark:text-blue-400",
                                        "{version}"
                                    }
                                }
                                
                                Dropdown {
                                    items: available_versions.read().iter().map(|version| {
                                        DropdownItem::new(version.clone(), version.clone())
                                            .with_description(format!("PDF version {}", version))
                                    }).collect(),
                                    selected: current_version.read().clone(),
                                    onselect: move |version: String| {
                                        let mut current_ver = current_version.clone();
                                        spawn_local(async move {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                use latex_ide_git_transport::web::WebGitTransport;
                                                
                                                let git_transport = WebGitTransport::new();
                                                match git_transport.rollback_to_version(version.clone()).await {
                                                    Ok(_) => {
                                                        current_ver.set(Some(version));
                                                        tracing::info!("Successfully reverted to version");
                                                        // Trigger PDF refresh by recompiling
                                                        // TODO: Get the LaTeX content for this version and recompile
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to revert to version: {}", e);
                                                    }
                                                }
                                            }
                                            
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                tracing::info!("Version rollback not implemented for non-wasm32 target");
                                            }
                                        });
                                    },
                                    placeholder: "Select version".to_string(),
                                    size: Some(DropdownSize::Small),
                                    variant: Some(DropdownVariant::Default),
                                }
                            }
                        }
                    }
                    
                    div { class: "flex items-center space-x-2 text-sm text-gray-600 dark:text-gray-400",
                        span { "PDF Preview" }
                        if let Some(version) = current_version.read().as_ref() {
                            span {
                                class: "text-xs font-mono bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 px-2 py-1 rounded",
                                "{version}"
                            }
                        }
                    }
                }
            }
            
            // PDF viewer area
            div { class: "flex-1 overflow-auto p-4",
                match *compilation_status.read() {
                    CompilationStatus::Compiling => {
                        rsx! {
                            div { class: "h-full flex items-center justify-center text-gray-500 dark:text-gray-400",
                                div { class: "text-center",
                                    div { class: "inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mb-4" }
                                    div { class: "text-lg mb-2", "Compiling LaTeX..." }
                                    if let Some(file_name) = current_file_name.as_ref() {
                                        div { class: "text-sm", "Processing: {file_name}" }
                                    }
                                }
                            }
                        }
                    }
                    CompilationStatus::Success => {
                        if let Some(url) = pdf_url.read().as_ref() {
                            rsx! {
                                iframe {
                                    src: "{url}",
                                    class: "w-full h-full border rounded-lg shadow-lg",
                                    "PDF Preview"
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "h-full flex items-center justify-center text-gray-500 dark:text-gray-400",
                                    div { class: "text-center",
                                        div { class: "text-6xl mb-4", "✅" }
                                        div { class: "text-lg mb-2", "Compilation Successful" }
                                        div { class: "text-sm", "PDF generated successfully" }
                                    }
                                }
                            }
                        }
                    }
                    CompilationStatus::Error => {
                        rsx! {
                            div { class: "h-full flex items-center justify-center text-red-500 dark:text-red-400",
                                div { class: "text-center max-w-md",
                                    div { class: "text-6xl mb-4", "❌" }
                                    div { class: "text-lg mb-2", "Backend Server Required" }
                                    div { class: "text-sm mb-4", "LaTeX compilation requires the backend server" }
                                    div { class: "text-xs text-left bg-red-50 dark:bg-red-900 p-3 rounded",
                                        "To enable PDF compilation:"
                                        ul { class: "list-disc list-inside mt-2 space-y-1",
                                            li { "Run: ./scripts/dev.sh web" }
                                            li { "Or: ./scripts/dev.sh backend" }
                                            li { "Server provides LaTeX compilation service" }
                                            li { "Includes file operations and git integration" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    CompilationStatus::Ready => {
                        if has_content {
                            rsx! {
                                div { class: "h-full flex items-center justify-center text-gray-500 dark:text-gray-400",
                                    div { class: "text-center",
                                        div { class: "text-6xl mb-4", "📄" }
                                        div { class: "text-lg mb-2", "Ready to Compile" }
                                        if let Some(file_name) = current_file_name.as_ref() {
                                            div { class: "text-sm mb-2", "File: {file_name}" }
                                        }
                                        div { class: "text-sm", "Click the compile button to generate PDF" }
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "h-full flex items-center justify-center text-gray-500 dark:text-gray-400",
                                    div { class: "text-center",
                                        div { class: "text-6xl mb-4", "📝" }
                                        div { class: "text-lg mb-2", "No Document Selected" }
                                        div { class: "text-sm", "Select a LaTeX file from the workspace to preview" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// TODO: Remove these helper functions once real WebTransport compilation is implemented

#[cfg(target_arch = "wasm32")]
fn create_pdf_url(pdf_data: Vec<u8>) -> Option<String> {
    use js_sys::Uint8Array;
    
    let array = Uint8Array::from(&pdf_data[..]);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array);
    
    // Create blob with explicit PDF MIME type
    let blob_init = web_sys::BlobPropertyBag::new();
    blob_init.set_type("application/pdf");
    
    if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &blob_init) {
        if let Ok(object_url) = web_sys::Url::create_object_url_with_blob(&blob) {
            return Some(object_url);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn create_pdf_url(_pdf_data: Vec<u8>) -> Option<String> {
    // Desktop implementation would save to file and return file:// URL
    None
}

#[cfg(target_arch = "wasm32")]
fn compile_latex(
    content: String,
    file_name: String,
    mut status: Signal<CompilationStatus>,
    mut pdf_url: Signal<Option<String>>,
    mut available_versions: Signal<Vec<String>>,
    mut current_version: Signal<Option<String>>
) {
    use serde::{Serialize, Deserialize};
    
    status.set(CompilationStatus::Compiling);
    
    spawn_local(async move {
        tracing::info!("Starting LaTeX compilation for: {}", file_name);
        
        #[derive(Serialize, Deserialize, Debug, Clone)]
        enum LocalTransportMessage {
            CompilationRequest { document_id: String, content: String, engine: String },
            CompilationResult { document_id: String, success: bool, pdf_data: Option<Vec<u8>>, log: String },
        }
        
        // Create WebSocket connection for compilation
        match web_sys::WebSocket::new("ws://localhost:3001/ws") {
            Ok(ws) => {
                ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
                
                // Wait for connection
                let connected = std::rc::Rc::new(std::cell::RefCell::new(false));
                let connected_clone = connected.clone();
                
                let onopen = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                    *connected_clone.borrow_mut() = true;
                    tracing::info!("WebSocket connected for compilation");
                }) as Box<dyn FnMut()>);
                ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                
                // Set up response handler
                let mut status_clone = status.clone();
                let mut pdf_clone = pdf_url.clone();
                let file_name_for_message = file_name.clone(); // Clone for the message handler
                let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                    if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                        let data = uint8_array.to_vec();
                        
                        match serde_json::from_slice::<LocalTransportMessage>(&data) {
                            Ok(LocalTransportMessage::CompilationResult { success, pdf_data, log, .. }) => {
                                if success {
                                    if let Some(pdf_bytes) = pdf_data {
                                        // Create PDF URL
                                        if let Some(url) = create_pdf_url(pdf_bytes) {
                                            pdf_clone.set(Some(url));
                                            
                                            // Auto-commit with versioning after successful compilation
                                            tracing::info!("PDF compiled successfully, creating version commit");
                                            
                                            // Clone variables for the closure
                                            let file_name = file_name_for_message.clone(); // Clone file_name for the closure
                                            let mut versions = available_versions.clone();
                                            let mut current_ver = current_version.clone();
                                            
                                            spawn_local(async move {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    use latex_ide_git_transport::web::WebGitTransport;
                                                    
                                                    let git_transport = WebGitTransport::new();
                                                    match git_transport.commit_pdf_version(
                                                        format!("output.pdf"),
                                                        format!("Generated from {}", file_name)
                                                    ).await {
                                                        Ok(commit) => {
                                                            tracing::info!("Successfully created PDF version commit: {} - {}", 
                                                                commit.short_id, commit.message);
                                                            
                                                            // Refresh the available versions list
                                                            match git_transport.get_all_versions().await {
                                                                Ok(version_list) => {
                                                                    versions.set(version_list.clone());
                                                                    // Set current version to the latest
                                                                    if let Some(latest) = version_list.last() {
                                                                        current_ver.set(Some(latest.clone()));
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::error!("Failed to refresh version list: {}", e);
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to create PDF version commit: {}", e);
                                                        }
                                                    }
                                                }
                                                
                                                #[cfg(not(target_arch = "wasm32"))]
                                                {
                                                    tracing::info!("PDF version commit not implemented for non-wasm32 target");
                                                }
                                            });
                                        }
                                    }
                                    status_clone.set(CompilationStatus::Success);
                                    tracing::info!("LaTeX compilation successful");
                                } else {
                                    status_clone.set(CompilationStatus::Error);
                                    tracing::error!("LaTeX compilation failed: {}", log);
                                }
                            }
                            _ => {}
                        }
                    }
                }) as Box<dyn FnMut(web_sys::MessageEvent)>);
                ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                
                // Wait for connection
                let mut attempts = 0;
                while attempts < 50 && !*connected.borrow() {
                    gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }
                
                if *connected.borrow() {
                    // Send compilation request
                    let request = LocalTransportMessage::CompilationRequest {
                        document_id: file_name.clone(),
                        content: content.clone(),
                        engine: "pdflatex".to_string(),
                    };
                    
                    match serde_json::to_vec(&request) {
                        Ok(data) => {
                            let array = js_sys::Uint8Array::from(&data[..]);
                            if let Err(e) = ws.send_with_array_buffer(&array.buffer()) {
                                tracing::error!("Failed to send compilation request: {:?}", e);
                                status.set(CompilationStatus::Error);
                            } else {
                                tracing::info!("Compilation request sent successfully");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize compilation request: {}", e);
                            status.set(CompilationStatus::Error);
                        }
                    }
                } else {
                    tracing::error!("Failed to connect to compilation server");
                    status.set(CompilationStatus::Error);
                }
                
                // Prevent closures from being dropped
                onopen.forget();
                onmessage.forget();
            }
            Err(e) => {
                tracing::error!("Failed to create WebSocket for compilation: {:?}", e);
                status.set(CompilationStatus::Error);
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn compile_latex(
    _content: String,
    _file_name: String,
    mut status: Signal<CompilationStatus>,
    _pdf_url: Signal<Option<String>>,
    _available_versions: Signal<Vec<String>>,
    _current_version: Signal<Option<String>>
) {
    // Desktop implementation would use local LaTeX compiler
    status.set(CompilationStatus::Error);
}


#[derive(Debug, Clone)]
struct SessionCommit {
    short_hash: String,
    message: String,
    version: Option<String>,
    timestamp: String,
}

#[component]
fn SessionHistoryBar(
    #[props(default)] session_manager: Option<Signal<Option<String>>>,
) -> Element {
    let commits = use_signal(|| Vec::<SessionCommit>::new());
    let mut selected_commit = use_signal(|| None::<String>);
    
    // Load commit history when component mounts or session manager changes
    use_effect(move || {
        if let Some(session_mgr) = session_manager {
            if let Some(session_name) = session_mgr.read().as_ref().cloned() {
                let mut commits = commits.clone();
                
                spawn_local(async move {
                    #[cfg(target_arch = "wasm32")]
                    {
                        // Try to get commit history via git transport
                        use latex_ide_git_transport::web::WebGitTransport;
                        
                        let git_transport = WebGitTransport::new();
                        match git_transport.get_commit_history(Some(session_name), Some(10)).await {
                            Ok(history) => {
                                let session_commits: Vec<SessionCommit> = history.into_iter()
                                    .map(|commit| {
                                        let message = commit.message.clone();
                                        SessionCommit {
                                            short_hash: commit.short_id,
                                            version: if message.contains("PDF version") {
                                                Some(message.split("PDF version ").nth(1).unwrap_or("v1.0.0").to_string())
                                            } else {
                                                None
                                            },
                                            message,
                                            timestamp: format_timestamp_from_iso(&commit.timestamp),
                                        }
                                    })
                                    .collect();
                                commits.set(session_commits);
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch commit history: {}", e);
                            }
                        }
                    }
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        tracing::info!("Commit history loading not implemented for non-wasm32 target");
                    }
                });
            }
        }
    });

    rsx! {
        div { class: "bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 px-4 py-2",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center space-x-2",
                    span { class: "text-xs font-medium text-gray-600 dark:text-gray-300",
                        "Session History:"
                    }
                    if !commits.read().is_empty() {
                        Dropdown {
                            items: commits.read().iter().map(|commit| {
                                DropdownItem::new(commit.short_hash.clone(), commit.short_hash.clone())
                                    .with_description(format!("{} - {}", 
                                        commit.version.as_deref().unwrap_or("No version"),
                                        commit.message.chars().take(30).collect::<String>()
                                    ))
                            }).collect(),
                            selected: selected_commit.read().clone(),
                            onselect: move |hash: String| {
                                selected_commit.set(Some(hash));
                            },
                            placeholder: "Select commit".to_string(),
                            size: Some(DropdownSize::Small),
                            variant: Some(DropdownVariant::Default),
                        }
                    } else {
                        span { class: "text-xs text-gray-500 dark:text-gray-400",
                            if session_manager.is_some() { "Loading..." } else { "No session" }
                        }
                    }
                }
                
                if let Some(hash) = selected_commit.read().clone() {
                    button {
                        class: "px-2 py-1 bg-orange-600 hover:bg-orange-700 text-white rounded text-xs font-medium transition-colors",
                        onclick: move |_| {
                            let commit_hash = hash.clone();
                            spawn_local(async move {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use latex_ide_git_transport::web::WebGitTransport;
                                    
                                    let git_transport = WebGitTransport::new();
                                    match git_transport.safe_rollback_to_commit(commit_hash).await {
                                        Ok(_) => {
                                            tracing::info!("Successfully rolled back to commit");
                                            // Could trigger PDF refresh here
                                        }
                                        Err(e) => {
                                            tracing::error!("Rollback failed: {}", e);
                                        }
                                    }
                                }
                                
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    tracing::info!("Rollback not implemented for non-wasm32 target");
                                }
                            });
                        },
                        "Rollback"
                    }
                }
            }
        }
    }
}

fn format_timestamp_from_iso(timestamp_iso: &str) -> String {
    // Parse ISO timestamp and format it for display
    match chrono::DateTime::parse_from_rfc3339(timestamp_iso) {
        Ok(timestamp) => format_timestamp(timestamp.with_timezone(&Utc)),
        Err(_) => timestamp_iso.to_string()
    }
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);
    
    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration.num_days() < 7 {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else {
        timestamp.format("%b %d, %Y").to_string()
    }
}