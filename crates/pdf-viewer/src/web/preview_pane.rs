use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::use_signal;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    gloo_timers,
    js_sys,
    web_sys,
};

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
    current_file: Option<Signal<Option<String>>>
) -> Element {
    let mut compilation_status = use_signal(|| CompilationStatus::Ready);
    let pdf_url = use_signal(|| None::<String>);
    
    let has_content = !document_content.read().is_empty();
    let current_file_name = current_file
        .and_then(|cf| cf.read().as_ref().and_then(|path| 
            std::path::Path::new(path).file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
        ));
    let current_file_name_for_closure = current_file_name.clone();
    
    rsx! {
        div { class: "h-full bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 flex flex-col",
            
            // PDF controls
            div { class: "h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4 bg-white dark:bg-gray-800",
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
                            let mut _status = compilation_status.clone();
                            let mut _pdf = pdf_url.clone();
                            let _file_name = current_file_name_for_closure.clone().unwrap_or_else(|| "document".to_string());
                            
                            #[cfg(target_arch = "wasm32")]
                            spawn_local(async move {
                                tracing::info!("Starting LaTeX compilation for: {}", _file_name);
                                
                                // Import transport types
                                use serde::{Serialize, Deserialize};
                                
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
                                        let mut status_clone = _status.clone();
                                        let mut pdf_clone = _pdf.clone();
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
                                                document_id: _file_name.clone(),
                                                content: _content.clone(),
                                                engine: "pdflatex".to_string(),
                                            };
                                            
                                            match serde_json::to_vec(&request) {
                                                Ok(data) => {
                                                    let array = js_sys::Uint8Array::from(&data[..]);
                                                    if let Err(e) = ws.send_with_array_buffer(&array.buffer()) {
                                                        tracing::error!("Failed to send compilation request: {:?}", e);
                                                        _status.set(CompilationStatus::Error);
                                                    } else {
                                                        tracing::info!("Compilation request sent successfully");
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to serialize compilation request: {}", e);
                                                    _status.set(CompilationStatus::Error);
                                                }
                                            }
                                        } else {
                                            tracing::error!("Failed to connect to compilation server");
                                            _status.set(CompilationStatus::Error);
                                        }
                                        
                                        // Prevent closures from being dropped
                                        onopen.forget();
                                        onmessage.forget();
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to create WebSocket for compilation: {:?}", e);
                                        _status.set(CompilationStatus::Error);
                                    }
                                }
                            });
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
                }
                
                div { class: "flex items-center space-x-2 text-sm text-gray-600 dark:text-gray-400",
                    span { "PDF Preview" }
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
                                    div { class: "text-lg mb-2", "Compilation Not Available" }
                                    div { class: "text-sm mb-4", "WebTransport compilation not implemented yet" }
                                    div { class: "text-xs text-left bg-red-50 dark:bg-red-900 p-3 rounded",
                                        "Required implementation:"
                                        ul { class: "list-disc list-inside mt-2 space-y-1",
                                            li { "WebTransport CompilationRequest messages" }
                                            li { "Server-side LaTeX compilation service" }
                                            li { "PDF result streaming via transport" }
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