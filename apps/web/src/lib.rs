use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use wasm_bindgen::prelude::*;

mod components;
mod hooks;
mod state;
mod transport;

use hooks::*;

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
    
    rsx! {
        ThemeProvider {
            div {
                id: "app",
                class: "h-screen w-screen bg-white dark:bg-gray-900 flex flex-col p-4",
                
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-4",
                    "LaTeX IDE Web"
                }
                
                div { class: "flex-1 flex gap-4",
                    div { class: "flex-1",
                        WebEditorPane { document_content: document_content }
                    }
                    
                    div { class: "flex-1",
                        WebPreviewPane { document_content: document_content }
                    }
                }
            }
        }
    }
}

#[component]
fn ConnectionBanner(capabilities: Signal<BrowserCapabilities>) -> Element {
    let caps = capabilities.read();
    
    if !caps.webtransport_supported && !caps.websocket_supported {
        rsx! {
            div { class: "bg-red-500 text-white px-4 py-2 text-sm",
                "⚠️ Limited connectivity: Your browser doesn't support WebTransport or WebSockets. Some features may not work."
            }
        }
    } else if !caps.webtransport_supported {
        rsx! {
            div { class: "bg-yellow-500 text-white px-4 py-2 text-sm",
                "⚡ Using WebSocket fallback. For best performance, use a browser that supports WebTransport."
            }
        }
    } else {
        rsx! { }
    }
}

#[component]
fn WebFileTree() -> Element {
    let files = use_signal(|| vec![
        ("document.tex".to_string(), true),
        ("figures/".to_string(), false),
        ("references.bib".to_string(), true),
        ("styles.sty".to_string(), true),
    ]);
    
    rsx! {
        div { class: "h-full overflow-auto p-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                "Project Files"
            }
            
            div { class: "space-y-1",
                {
                    files.read().iter().enumerate().map(|(i, (name, is_file))| {
                        let name = name.clone();
                        let is_file = *is_file;
                        rsx! {
                            WebFileItem { 
                                key: "{i}",
                                name: name.clone(),
                                is_file: is_file,
                                onclick: move |_| {
                                    tracing::info!("Clicked file: {}", name);
                                }
                            }
                        }
                    })
                }
            }
            
            // Add file/folder buttons
            div { class: "mt-4 pt-4 border-t border-gray-200 dark:border-gray-700",
                div { class: "flex space-x-2",
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Create new file
                        },
                        "📄 New File"
                    }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Upload file
                        },
                        "📤 Upload"
                    }
                }
            }
        }
    }
}

#[component]
fn WebFileItem(name: String, is_file: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let icon = if is_file { "📄" } else { "📁" };
    
    rsx! {
        div { 
            class: "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer transition-colors",
            onclick: move |evt| onclick.call(evt),
            
            span { "{icon}" }
            span { "{name}" }
        }
    }
}

#[component]
fn WebEditorPane(document_content: Signal<String>) -> Element {
    
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
            
            // Web-optimized editor  
            div { class: "flex-1 relative",
                WebTextEditor {
                    content: document_content,
                }
            }
        }
    }
}

#[component]
fn WebTextEditor(content: Signal<String>) -> Element {
    let mut buffer = use_signal(|| latex_ide_editor::TextBuffer::from_str(&content.read()));
    let mut cursor = use_signal(|| latex_ide_editor::Cursor::new());
    let mut highlighter = use_signal(|| latex_ide_editor::SyntaxHighlighter::new());
    let mut executor = use_signal(|| latex_ide_editor::commands::CommandExecutor::new());
    
    // Update buffer when external content changes
    use_effect(move || {
        let current_content = buffer.read().get_text();
        let new_content = content.read();
        if current_content != *new_content {
            buffer.set(latex_ide_editor::TextBuffer::from_str(&new_content));
            highlighter.write().parse(&new_content);
        }
    });
    
    // Parse content for syntax highlighting
    use_effect(move || {
        highlighter.write().parse(&buffer.read().get_text());
    });
    
    let handle_keydown = move |evt: KeyboardEvent| {
        let key = evt.key();
        let ctrl = evt.modifiers().ctrl();
        
        let command = match key {
            Key::Character(ch) if !ctrl => {
                Some(latex_ide_editor::commands::EditorCommand::InsertChar(ch.chars().next().unwrap_or(' ')))
            }
            Key::Backspace => Some(latex_ide_editor::commands::EditorCommand::Backspace),
            Key::Delete => Some(latex_ide_editor::commands::EditorCommand::DeleteChar),
            Key::ArrowLeft => Some(latex_ide_editor::commands::EditorCommand::MoveCursorLeft),
            Key::ArrowRight => Some(latex_ide_editor::commands::EditorCommand::MoveCursorRight),
            Key::ArrowUp => Some(latex_ide_editor::commands::EditorCommand::MoveCursorUp),
            Key::ArrowDown => Some(latex_ide_editor::commands::EditorCommand::MoveCursorDown),
            Key::Home => Some(latex_ide_editor::commands::EditorCommand::MoveToLineStart),
            Key::End => Some(latex_ide_editor::commands::EditorCommand::MoveToLineEnd),
            Key::Character(ch) if ctrl && ch == "z" => Some(latex_ide_editor::commands::EditorCommand::Undo),
            Key::Character(ch) if ctrl && ch == "y" => Some(latex_ide_editor::commands::EditorCommand::Redo),
            _ => None,
        };
        
        if let Some(cmd) = command {
            executor.write().execute(cmd, &mut buffer.write(), &mut cursor.write());
            
            // Update parent content
            let new_content = buffer.read().get_text();
            content.set(new_content.clone());
            
            // Re-parse for syntax highlighting
            highlighter.write().parse(&new_content);
        }
    };
    
    rsx! {
        div {
            class: "flex h-full w-full bg-white dark:bg-gray-900 font-mono text-sm",
            
            // Line numbers
            div {
                class: "select-none px-4 py-2 text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700",
                for line_num in 1..=buffer.read().len_lines() {
                    div {
                        class: "text-right leading-6",
                        "{line_num}"
                    }
                }
            }
            
            // Editor content
            div {
                class: "flex-1 relative",
                tabindex: 0,
                onkeydown: handle_keydown,
                
                // Text content with syntax highlighting
                div {
                    class: "p-2",
                    for (line_idx, line) in buffer.read().get_text().lines().enumerate() {
                        div {
                            class: "min-h-[1.5rem] leading-6",
                            RenderHighlightedLine {
                                line: line.to_string(),
                                line_idx: line_idx,
                                highlighter: highlighter,
                            }
                        }
                    }
                }
                
                // Cursor
                RenderCursor {
                    cursor: cursor,
                    buffer: buffer,
                }
            }
        }
    }
}

#[component]
fn RenderHighlightedLine(
    line: String,
    line_idx: usize,
    highlighter: Signal<latex_ide_editor::SyntaxHighlighter>,
) -> Element {
    let highlights = highlighter.read().get_highlights(line_idx, line_idx);
    
    rsx! {
        span {
            if highlights.is_empty() {
                span {
                    class: "text-gray-900 dark:text-gray-100",
                    "{line}"
                }
            } else {
                for highlight in highlights {
                    span {
                        class: "{highlight.highlight_type.to_class()}",
                        "{&line[highlight.start_col..highlight.end_col.min(line.len())]}"
                    }
                }
            }
        }
    }
}

#[component]
fn RenderCursor(
    cursor: Signal<latex_ide_editor::Cursor>,
    buffer: Signal<latex_ide_editor::TextBuffer>,
) -> Element {
    let cursor_pos = cursor.read().position;
    let line_height = 1.5; // rem
    let char_width = 0.6; // rem
    
    let top = cursor_pos.line as f32 * line_height;
    let left = cursor_pos.column as f32 * char_width;
    
    rsx! {
        div {
            class: "absolute w-0.5 h-6 bg-blue-600 animate-pulse",
            style: "top: {top}rem; left: {left}rem;",
        }
    }
}

#[component] 
fn WebPreviewPane(document_content: Signal<String>) -> Element {
    let mut compilation_status = use_signal(|| CompilationStatus::Ready);
    let pdf_url = use_signal(|| None::<String>);
    let mut transport = use_signal(|| None::<crate::transport::Transport>);
    
    // Initialize transport connection
    use_effect(move || {
        wasm_bindgen_futures::spawn_local(async move {
            let mut t = crate::transport::Transport::new();
            match t.connect("ws://localhost:3001").await {
                Ok(_) => {
                    tracing::info!("Transport connected for compilation");
                    transport.set(Some(t));
                }
                Err(e) => {
                    tracing::error!("Transport connection failed: {:?}", e);
                }
            }
        });
    });
    
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
                            
                            wasm_bindgen_futures::spawn_local(async move {
                                tracing::info!("Starting LaTeX compilation");
                                
                                // Send compilation request to backend
                                let compilation_request = crate::transport::TransportMessage::CompilationRequest {
                                    document_id: "main".to_string(),
                                    content: content,
                                    engine: "pdflatex".to_string(),
                                };
                                
                                // Try to send via WebSocket to localhost:3001
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

#[component]
fn WebAIChatInterface(capabilities: Signal<BrowserCapabilities>) -> Element {
    let available_models = use_signal(|| vec![
        "GPT-4o (OpenAI)".to_string(),
        "Claude 3.5 Sonnet".to_string(), 
        "Gemini Pro".to_string(),
        "Llama 3.1 (via Ollama)".to_string(),
    ]);
    
    let mut selected_model = use_signal(|| "GPT-4o (OpenAI)".to_string());
    let mut chat_messages = use_signal(|| Vec::<ChatMessage>::new());
    let mut current_message = use_signal(|| String::new());
    let mut is_streaming = use_signal(|| false);
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // AI Chat header
            div { class: "p-4 border-b border-gray-200 dark:border-gray-700",
                h3 { class: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-3",
                    "AI Assistant"
                }
                
                // Model selection
                Dropdown {
                    items: available_models.read().iter().enumerate().map(|(i, model)| {
                        DropdownItem { 
                            id: i.to_string(), 
                            label: model.clone(),
                            icon: None 
                        }
                    }).collect(),
                    selected: Some("0".to_string()),
                    onselect: move |model_idx: String| {
                        if let Ok(idx) = model_idx.parse::<usize>() {
                            if let Some(model) = available_models.read().get(idx) {
                                selected_model.set(model.clone());
                            }
                        }
                    },
                    placeholder: "Select Model".to_string(),
                }
                
                // Connection status
                if capabilities.read().server_sent_events {
                    div { class: "text-xs text-green-600 dark:text-green-400 mt-2",
                        "🟢 Connected via Server-Sent Events"
                    }
                } else {
                    div { class: "text-xs text-yellow-600 dark:text-yellow-400 mt-2",
                        "🟡 Limited AI connectivity"
                    }
                }
            }
            
            // Chat messages
            div { class: "flex-1 overflow-auto p-4 space-y-4",
                if chat_messages.read().is_empty() {
                    div { class: "text-center text-gray-500 dark:text-gray-400 mt-8",
                        div { class: "text-4xl mb-4", "🤖" }
                        div { class: "text-lg mb-2", "AI Assistant Ready" }
                        div { class: "text-sm",
                            "Ask me about LaTeX syntax, document structure,"
                            br {}
                            "mathematical typesetting, or get writing help."
                        }
                    }
                } else {
                    for message in chat_messages.read().iter() {
                        ChatBubble { message: message.clone() }
                    }
                }
                
                if *is_streaming.read() {
                    div { class: "flex items-center space-x-2 text-gray-500 dark:text-gray-400",
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full" }
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full", style: "animation-delay: 0.2s" }
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full", style: "animation-delay: 0.4s" }
                        span { class: "text-sm", "AI is thinking..." }
                    }
                }
            }
            
            // Message input
            div { class: "p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800",
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
                        disabled: is_streaming.read().clone() || current_message.read().trim().is_empty(),
                        onclick: move |_| {
                            let message = current_message.read().clone();
                            if !message.trim().is_empty() {
                                // Add user message
                                chat_messages.write().push(ChatMessage {
                                    content: message,
                                    is_user: true,
                                    timestamp: js_sys::Date::now(),
                                });
                                current_message.set(String::new());
                                is_streaming.set(true);
                                
                                // Simulate AI response (replace with actual SSE)
                                wasm_bindgen_futures::spawn_local(async move {
                                    gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                                    chat_messages.write().push(ChatMessage {
                                        content: "I can help you with that LaTeX question! Here's what I suggest...".to_string(),
                                        is_user: false,
                                        timestamp: js_sys::Date::now(),
                                    });
                                    is_streaming.set(false);
                                });
                            }
                        },
                        if *is_streaming.read() { "..." } else { "Send" }
                    }
                }
            }
        }
    }
}

#[component]
fn ChatBubble(message: ChatMessage) -> Element {
    let bg_class = if message.is_user {
        "bg-blue-500 text-white ml-auto"
    } else {
        "bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100 mr-auto"
    };
    
    rsx! {
        div { 
            class: "max-w-xs lg:max-w-md px-4 py-2 rounded-lg {bg_class}",
            "{message.content}"
        }
    }
}

#[component]
fn WebStatusBar(capabilities: Signal<BrowserCapabilities>) -> Element {
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

#[derive(Clone, Debug, PartialEq)]
struct ChatMessage {
    content: String,
    is_user: bool,
    timestamp: f64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum CompilationStatus {
    Ready,
    Compiling,
    Success,
    Error,
}

// Helper functions for compilation  
async fn send_compilation_request(_request: crate::transport::TransportMessage) -> Result<Option<Vec<u8>>, JsValue> {
    // For now, create a simple mock PDF response for testing
    tracing::info!("Mock compilation - generating sample PDF");
    
    // Simulate async compilation delay
    gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
    
    // Create a minimal PDF content for testing
    let sample_pdf_content = create_sample_pdf();
    
    Ok(Some(sample_pdf_content))
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