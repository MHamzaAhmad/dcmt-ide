use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::use_signal;
use wasm_bindgen::prelude::*;

mod components;
mod hooks;
mod state;
mod transport;

use hooks::*;

// Entry point for WASM
#[wasm_bindgen(start)]
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
                        WebEditorPane { }
                    }
                    
                    div { class: "flex-1",
                        WebPreviewPane { }
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
fn WebEditorPane() -> Element {
    let mut document_content = use_signal(|| "% LaTeX document\n\\documentclass{article}\n\\begin{document}\nHello World!\n\\end{document}".to_string());
    
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
                latex_ide_editor::TextEditor {
                    initial_content: Some(document_content.read().clone()),
                    onchange: EventHandler::new(move |content| {
                        document_content.set(content);
                    }),
                    show_line_numbers: true,
                    syntax_highlighting: true,
                }
            }
        }
    }
}

#[component] 
fn WebPreviewPane() -> Element {
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
                            // Trigger compilation via WebTransport
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