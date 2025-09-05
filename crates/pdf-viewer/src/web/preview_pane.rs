use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use latex_ide_ui::*;
use latex_ide_ui::web::transport::{use_connection_manager, ConnectionManager, ConnectionState, TransportType, TransportMessage};
#[cfg(feature = "native-git")]
use latex_ide_git_manager::{SessionManager, GitRepository, HistoryViewer};
use chrono::{DateTime, Utc};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen_futures::spawn_local,
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
    
    // Initialize connection manager
    let connection_manager = use_connection_manager("localhost:3001".to_string());
    let connection_manager_clone1 = connection_manager.clone();
    let connection_manager_clone2 = connection_manager.clone();
    let _connection_manager_clone3 = connection_manager.clone();
    let connection_manager_clone4 = connection_manager.clone();
    let connection_manager_clone5 = connection_manager.clone();
    let connection_manager_clone6 = connection_manager.clone();
    
    let has_content = !document_content.read().is_empty();
    let current_file_name = current_file
        .and_then(|cf| cf.read().as_ref().and_then(|path| 
            std::path::Path::new(path).file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
        ));
    
    // Clone the file name for use in closures
    let current_file_name_for_closures = current_file_name.clone();
    let current_file_name_for_trigger = current_file_name.clone();
    let current_file_name_for_compile = current_file_name.clone();
    let current_file_name_for_dropdown = current_file_name.clone();
    
    // Proactive connection establishment when component mounts
    use_effect(move || {
        tracing::info!("📄 PDF viewer component mounted, initializing backend connection");
        let mut connection_manager = connection_manager_clone5.clone();
        
        spawn_local(async move {
            #[cfg(target_arch = "wasm32")]
            {
                tracing::info!("🔄 PDF viewer starting proactive backend connection (1 retry max)");
                
                match connection_manager.connect_with_retry(1).await {
                    Ok(_) => {
                        tracing::info!("🎉 PDF viewer successfully connected to backend server");
                        
                        // Log current connection state for debugging
                        let state = *connection_manager.connection_state().read();
                        let transport = *connection_manager.transport_type().read();
                        tracing::debug!("📊 Final connection status - State: {:?}, Transport: {:?}", state, transport);
                    }
                    Err(e) => {
                        tracing::error!("❌ PDF viewer failed to connect to backend server: {}", e);
                        
                        // Log detailed failure info
                        let state = *connection_manager.connection_state().read();
                        tracing::debug!("📊 Failed connection status - State: {:?}", state);
                        tracing::info!("💡 Connection will be retried when LaTeX compilation is requested");
                    }
                }
            }
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                tracing::debug!("🖥️  PDF viewer on non-WASM platform, skipping proactive connection");
            }
        });
    });
    
    // Load available versions when session is established
    use_effect(move || {
        if let Some(session_signal) = session_manager.as_ref() {
            if let Some(session_name) = session_signal.read().as_ref() {
                let session_name = session_name.clone();
                tracing::info!("Loading versions for session: {}", session_name);
                let mut versions = available_versions;
                let mut current_ver = current_version;
            
            spawn_local(async move {
                #[cfg(target_arch = "wasm32")]
                {
                    use latex_ide_git_transport::web::WebGitTransport;
                    
                    let git_transport = WebGitTransport::new();
                    match git_transport.get_all_versions().await {
                        Ok(version_list) => {
                            versions.set(version_list.clone());
                            // Set current version to latest if available (first in the list is highest version number)
                            if let Some(latest) = version_list.first() {
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
            
            compile_latex_with_transport(content, file_name, status, pdf, available_versions.clone(), current_version.clone(), &connection_manager_clone1);
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
                
                compile_latex_with_transport(content, file_name, status, pdf, available_versions.clone(), current_version.clone(), &connection_manager_clone4);
            }
        });
    }
    let _current_file_name_for_closure = current_file_name.clone();
    
    rsx! {
        div { class: "h-full bg-zinc-50 dark:bg-zinc-950 border-l border-zinc-200 dark:border-zinc-800 flex flex-col",
            
            // PDF controls with version management
            div { class: "h-10 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 flex items-center justify-between px-4",
                div { class: "flex items-center space-x-2",
                    button {
                        class: if matches!(*compilation_status.read(), CompilationStatus::Compiling) || !has_content {
                            "px-3 py-1 text-xs font-medium text-zinc-400 dark:text-zinc-600 bg-zinc-100 dark:bg-zinc-800 rounded-md cursor-not-allowed"
                        } else {
                            match *compilation_status.read() {
                                CompilationStatus::Success => "px-3 py-1 text-xs font-medium text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-900 hover:bg-green-100 dark:hover:bg-green-800 rounded-md transition-colors",
                                CompilationStatus::Error => "px-3 py-1 text-xs font-medium text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900 hover:bg-red-100 dark:hover:bg-red-800 rounded-md transition-colors",
                                _ => "px-3 py-1 text-xs font-medium text-white dark:text-zinc-900 bg-zinc-900 dark:bg-zinc-100 hover:bg-zinc-800 dark:hover:bg-zinc-200 rounded-md transition-colors"
                            }
                        },
                        disabled: matches!(*compilation_status.read(), CompilationStatus::Compiling) || !has_content,
                        onclick: move |_| {
                            if !has_content {
                                return;
                            }
                            
                            compilation_status.set(CompilationStatus::Compiling);
                            
                            let _content = document_content.read().clone();
                            let status = compilation_status.clone();
                            let pdf = pdf_url.clone();
                            let file_name = current_file_name_for_compile.clone().unwrap_or_else(|| "document".to_string());
                            
                            compile_latex_with_transport(_content, file_name, status, pdf, available_versions.clone(), current_version.clone(), &connection_manager_clone2);
                        },
                        match *compilation_status.read() {
                            CompilationStatus::Compiling => "Compiling...",
                            CompilationStatus::Success => "Compiled",
                            CompilationStatus::Error => "Error",
                            CompilationStatus::Ready => if has_content { "Compile" } else { "No Content" }
                        }
                    }
                    
                    button {
                        class: "px-3 py-1 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-zinc-100 dark:hover:bg-zinc-700 rounded-md transition-colors",
                        onclick: move |_| {
                            // Download PDF
                            tracing::info!("Download PDF clicked");
                        },
                        "Download"
                    }
                    
                    // Version dropdown
                    if !available_versions.read().is_empty() {
                        div {
                            class: "flex items-center gap-2 ml-4 border-l border-zinc-200 dark:border-zinc-700 pl-4",
                            span {
                                class: "text-sm text-zinc-600 dark:text-zinc-400",
                                "Version:"
                            }
                            
                            Dropdown {
                                items: available_versions.read().iter().map(|version| {
                                    DropdownItem::new(version.clone(), version.clone())
                                        .with_description(format!("PDF version {}", version))
                                }).collect(),
                                selected: current_version.read().clone(),
                                onselect: move |version: String| {
                                    let mut current_ver = current_version.clone();
                                    let status = compilation_status.clone();
                                    let pdf = pdf_url.clone();
                                    let content = document_content.read().clone();
                                    let file_name = current_file_name_for_dropdown.clone().unwrap_or_else(|| "document".to_string());
                                    let connection_mgr = connection_manager_clone6.clone();
                                    
                                    spawn_local(async move {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            use latex_ide_git_transport::web::WebGitTransport;
                                            
                                            let git_transport = WebGitTransport::new();
                                            match git_transport.rollback_to_version(version.clone()).await {
                                                Ok(_) => {
                                                    current_ver.set(Some(version.clone()));
                                                    tracing::info!("Successfully reverted to version: {}", version);
                                                    
                                                    // After successful rollback, trigger PDF recompilation
                                                    // The rollback has updated the working directory, so we can recompile with current content
                                                    tracing::info!("Triggering PDF recompilation after version rollback");
                                                    compile_latex_with_transport(
                                                        content, 
                                                        file_name, 
                                                        status, 
                                                        pdf, 
                                                        available_versions.clone(), 
                                                        current_ver,
                                                        &connection_mgr
                                                    );
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
            }
            
            // PDF viewer area
            div { class: "flex-1 overflow-hidden",
                match *compilation_status.read() {
                    CompilationStatus::Compiling => {
                        rsx! {
                            div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
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
                                    src: "{url}#toolbar=0&navpanes=0&scrollbar=0",
                                    class: "w-full h-full border-0",
                                    "PDF Preview"
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
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
                        let connection_state_signal = connection_manager.connection_state();
                        let transport_type_signal = connection_manager.transport_type();
                        
                        rsx! {
                            div { class: "h-full flex items-center justify-center text-red-500 dark:text-red-400",
                                div { class: "text-center max-w-md",
                                    div { class: "text-6xl mb-4", "❌" }
                                    
                                    match *connection_state_signal.read() {
                                        ConnectionState::Disconnected => rsx! {
                                            div { class: "text-lg mb-2", "🔌 Backend Server Not Running" }
                                            div { class: "text-sm mb-4", "LaTeX compilation requires the backend server to be running on localhost:3001" }
                                            div { class: "text-xs text-yellow-600 dark:text-yellow-400 mb-4", "This usually means you need to start the development server" }
                                        },
                                        ConnectionState::ConnectingWebTransport => rsx! {
                                            div { class: "text-lg mb-2", "🌐 Connecting via WebTransport..." }
                                            div { class: "text-sm mb-4", "Attempting high-speed WebTransport connection" }
                                        },
                                        ConnectionState::ConnectingWebSocket => rsx! {
                                            div { class: "text-lg mb-2", "🔄 WebSocket Fallback..." }
                                            div { class: "text-sm mb-4", "WebTransport unavailable, trying WebSocket connection" }
                                        },
                                        ConnectionState::Failed => rsx! {
                                            div { class: "text-lg mb-2", "⚠️ Connection Failed" }
                                            div { class: "text-sm mb-4", "All connection attempts failed. Check if the backend server is running." }
                                            div { class: "text-xs text-orange-600 dark:text-orange-400 mb-4", "Network issues or server not responding on localhost:3001" }
                                        },
                                        ConnectionState::Connected => {
                                            let transport_name = match *transport_type_signal.read() {
                                                Some(TransportType::WebTransport) => "WebTransport",
                                                Some(TransportType::WebSocket) => "WebSocket",
                                                None => "Unknown"
                                            };
                                            rsx! {
                                                div { class: "text-lg mb-2", "📝 LaTeX Compilation Error" }
                                                div { class: "text-sm mb-4", "Connected via {transport_name} but LaTeX compilation failed" }
                                                div { class: "text-xs text-blue-600 dark:text-blue-400 mb-4", "Check your LaTeX syntax or compiler logs for details" }
                                            }
                                        }
                                    }
                                    
                                    div { class: "text-xs text-left bg-red-50 dark:bg-red-900 p-3 rounded",
                                        match *connection_state_signal.read() {
                                            ConnectionState::Disconnected | ConnectionState::Failed => rsx! {
                                                div { class: "mb-2", "📋 To start the backend server:" }
                                                ul { class: "list-disc list-inside mt-2 space-y-1",
                                                    li { "🚀 Run: ./scripts/dev.sh web" }
                                                    li { "Or: ./scripts/dev.sh backend" }
                                                    li { "📦 Provides LaTeX compilation with TinyTeX" }
                                                    li { "🔧 Includes file operations and git integration" }
                                                }
                                                div { class: "mt-3 pt-2 border-t border-red-200 dark:border-red-700 text-center",
                                                    "💡 The server needs to be running on localhost:3001"
                                                }
                                            },
                                            _ => rsx! {
                                                div { class: "mb-2", "🔄 Troubleshooting:" }
                                                ul { class: "list-disc list-inside mt-2 space-y-1",
                                                    li { "Check your LaTeX document syntax" }
                                                    li { "Look at browser console for error details" }
                                                    li { "Ensure required LaTeX packages are available" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    CompilationStatus::Ready => {
                        if has_content {
                            rsx! {
                                div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
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
                                div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
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
fn _create_pdf_url(_pdf_data: Vec<u8>) -> Option<String> {
    // Desktop implementation would save to file and return file:// URL
    None
}

/// New compilation function that uses the transport layer
fn compile_latex_with_transport(
    content: String,
    file_name: String,
    mut status: Signal<CompilationStatus>,
    pdf_url: Signal<Option<String>>,
    available_versions: Signal<Vec<String>>,
    current_version: Signal<Option<String>>,
    connection_manager: &ConnectionManager,
) {
    status.set(CompilationStatus::Compiling);
    
    #[cfg(target_arch = "wasm32")]
    {
        let mut connection_manager = connection_manager.clone(); // Clone for async
        spawn_local(async move {
            tracing::info!("Starting LaTeX compilation via transport layer for: {}", file_name);
            
            // Check connection state first
            let current_state = *connection_manager.connection_state().read();
            tracing::debug!("📊 Current connection state before compilation: {:?}", current_state);
            
            match current_state {
                ConnectionState::Disconnected => {
                    tracing::info!("🔌 Server disconnected, attempting to connect with retry (2 attempts)...");
                    
                    match connection_manager.connect_with_retry(2).await {
                        Ok(_) => {
                            let final_state = *connection_manager.connection_state().read();
                            let transport = *connection_manager.transport_type().read();
                            tracing::info!("🎉 Successfully connected for compilation - State: {:?}, Transport: {:?}", 
                                         final_state, transport);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to connect to server for compilation: {}", e);
                            
                            // Log detailed state for debugging
                            let failed_state = *connection_manager.connection_state().read();
                            tracing::debug!("📊 Failed connection state: {:?}", failed_state);
                            
                            status.set(CompilationStatus::Error);
                            return;
                        }
                    }
                }
                ConnectionState::Failed => {
                    tracing::error!("Connection is in failed state");
                    status.set(CompilationStatus::Error);
                    return;
                }
                _ => {}
            }
            
            // Setup message handler for responses
            let mut status_clone = status.clone();
            let mut pdf_clone = pdf_url.clone();
            let file_name_for_handler = file_name.clone();
            let versions_clone = available_versions.clone();
            let current_ver_clone = current_version.clone();
            
            let _ = connection_manager.setup_message_handler(move |message| {
                if let TransportMessage::CompilationResult { success, pdf_data, log, .. } = message {
                    if success {
                        if let Some(pdf_bytes) = pdf_data {
                            // Create PDF URL
                            if let Some(url) = create_pdf_url(pdf_bytes) {
                                pdf_clone.set(Some(url));
                                
                                // Auto-commit with versioning after successful compilation
                                tracing::info!("PDF compiled successfully, creating version commit");
                                
                                // Clone variables for the closure
                                let file_name = file_name_for_handler.clone();
                                let mut versions = versions_clone.clone();
                                let mut current_ver = current_ver_clone.clone();
                                
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
            });
            
            // Send compilation request
            let request = TransportMessage::CompilationRequest {
                document_id: file_name.clone(),
                content: content.clone(),
                engine: "pdflatex".to_string(),
            };
            
            match connection_manager.send_message(request).await {
                Ok(_) => {
                    tracing::info!("Compilation request sent successfully via transport layer");
                }
                Err(e) => {
                    tracing::error!("Failed to send compilation request: {}", e);
                    status.set(CompilationStatus::Error);
                }
            }
        });
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop implementation would use local LaTeX compiler
        tracing::info!("Desktop compilation not implemented yet");
        status.set(CompilationStatus::Error);
    }
}


fn _format_timestamp_from_iso(timestamp_iso: &str) -> String {
    // Parse ISO timestamp and format it for display
    match chrono::DateTime::parse_from_rfc3339(timestamp_iso) {
        Ok(timestamp) => _format_timestamp(timestamp.with_timezone(&Utc)),
        Err(_) => timestamp_iso.to_string()
    }
}

fn _format_timestamp(timestamp: DateTime<Utc>) -> String {
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