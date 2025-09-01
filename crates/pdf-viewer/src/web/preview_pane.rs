use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::use_signal;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    gloo_timers::future::sleep,
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
pub fn WebPreviewPane(document_content: Signal<String>) -> Element {
    let mut compilation_status = use_signal(|| CompilationStatus::Ready);
    let pdf_url = use_signal(|| None::<String>);
    
    rsx! {
        div { class: "h-full bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 flex flex-col",
            
            // PDF controls
            div { class: "h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4 bg-white dark:bg-gray-800",
                div { class: "flex items-center space-x-2",
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Small,
                        disabled: matches!(*compilation_status.read(), CompilationStatus::Compiling),
                        onclick: move |_| {
                            compilation_status.set(CompilationStatus::Compiling);
                            
                            let content = document_content.read().clone();
                            let mut status = compilation_status.clone();
                            let mut pdf = pdf_url.clone();
                            
                            #[cfg(target_arch = "wasm32")]
                            spawn_local(async move {
                                tracing::info!("Starting LaTeX compilation");
                                
                                // Send compilation request to backend
                                let compilation_request = CompilationRequest {
                                    document_id: "main".to_string(),
                                    content: content,
                                    engine: "pdflatex".to_string(),
                                };
                                
                                // Try to compile via mock for now
                                match send_compilation_request(compilation_request).await {
                                    Ok(pdf_data) => {
                                        tracing::info!("Compilation successful");
                                        status.set(CompilationStatus::Success);
                                        
                                        // Create object URL for PDF
                                        if let Some(data) = pdf_data {
                                            if let Some(url) = create_pdf_url(data) {
                                                pdf.set(Some(url));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Compilation failed: {:?}", e);
                                        status.set(CompilationStatus::Error);
                                    }
                                }
                            });
                        },
                        match *compilation_status.read() {
                            CompilationStatus::Compiling => "⏳ Compiling...",
                            CompilationStatus::Success => "✅ Compile",
                            CompilationStatus::Error => "❌ Compile",
                            CompilationStatus::Ready => "▶️ Compile"
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
                if let Some(url) = pdf_url.read().as_ref() {
                    iframe {
                        src: "{url}",
                        class: "w-full h-full border rounded-lg shadow-lg",
                        "PDF Preview"
                    }
                } else {
                    div { class: "h-full flex items-center justify-center text-gray-500 dark:text-gray-400",
                        div { class: "text-center",
                            div { class: "text-6xl mb-4", "📄" }
                            div { class: "text-lg mb-2", "No PDF Generated" }
                            div { class: "text-sm", "Compile your LaTeX document to see the preview" }
                        }
                    }
                }
            }
        }
    }
}

// Helper functions for compilation
#[cfg(target_arch = "wasm32")]
async fn send_compilation_request(_request: CompilationRequest) -> Result<Option<Vec<u8>>, JsValue> {
    // For now, create a simple mock PDF response for testing
    tracing::info!("Mock compilation - generating sample PDF");
    
    // Simulate async compilation delay
    sleep(std::time::Duration::from_secs(2)).await;
    
    // Create a minimal PDF content for testing
    let sample_pdf_content = create_sample_pdf();
    
    Ok(Some(sample_pdf_content))
}

#[cfg(not(target_arch = "wasm32"))]
async fn send_compilation_request(_request: CompilationRequest) -> Result<Option<Vec<u8>>, String> {
    // Desktop implementation would go here
    Err("Not implemented for desktop yet".to_string())
}

fn create_sample_pdf() -> Vec<u8> {
    // A minimal valid PDF for testing
    let pdf_content = r#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Hello World!) Tj
ET
endstream
endobj

xref
0 5
0000000000 65535 f 
0000000010 00000 n 
0000000079 00000 n 
0000000136 00000 n 
0000000215 00000 n 
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
310
%%EOF"#;
    
    pdf_content.as_bytes().to_vec()
}

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