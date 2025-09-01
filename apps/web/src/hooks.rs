use dioxus_signals::{Signal, Writable};
use dioxus_hooks::{use_signal, use_effect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::{self, Window, EventSource, MessageEvent, KeyboardEvent};
use yrs::{Doc as YDoc, Text, Transact};
use crate::transport::{Transport, ConnectionState};

/// Browser capabilities detection
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BrowserCapabilities {
    pub webtransport_supported: bool,
    pub websocket_supported: bool,
    pub server_sent_events: bool,
    pub web_assembly: bool,
    pub local_storage: bool,
    pub indexed_db: bool,
}

/// Hook to detect browser capabilities
#[allow(dead_code)]
pub fn use_browser_capabilities() -> Signal<BrowserCapabilities> {
    let mut capabilities = use_signal(|| BrowserCapabilities {
        webtransport_supported: false,
        websocket_supported: false,
        server_sent_events: false,
        web_assembly: true, // We're running in WASM
        local_storage: false,
        indexed_db: false,
    });
    
    use_effect(move || {
        wasm_bindgen_futures::spawn_local(async move {
            let window = web_sys::window().unwrap();
            
            let caps = BrowserCapabilities {
                webtransport_supported: check_webtransport_support(&window),
                websocket_supported: check_websocket_support(&window),
                server_sent_events: check_sse_support(&window),
                web_assembly: true,
                local_storage: check_local_storage_support(&window),
                indexed_db: check_indexeddb_support(&window),
            };
            
            capabilities.set(caps);
        });
    });
    
    capabilities
}

#[allow(dead_code)]
fn check_webtransport_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"WebTransport".into()).unwrap_or(false)
}

#[allow(dead_code)]
fn check_websocket_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"WebSocket".into()).unwrap_or(false)
}

#[allow(dead_code)]
fn check_sse_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"EventSource".into()).unwrap_or(false)
}

#[allow(dead_code)]
fn check_local_storage_support(window: &Window) -> bool {
    window.local_storage().is_ok()
}

#[allow(dead_code)]
fn check_indexeddb_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"indexedDB".into()).unwrap_or(false)
}

/// Hook for yrs document management
#[allow(dead_code)]
pub fn use_yrs_document() -> Signal<Option<YDoc>> {
    let mut doc = use_signal(|| None);
    
    use_effect(move || {
        // Initialize yrs document
        let ydoc = YDoc::new();
        
        // Get or create text field for document content
        let text = ydoc.get_or_insert_text("content");
        
        // Set initial content
        text.insert(&mut ydoc.transact_mut(), 0, "% LaTeX document\n\\documentclass{article}\n\\begin{document}\n\\end{document}");
        
        doc.set(Some(ydoc));
    });
    
    doc
}

/// Hook for transport connection management
#[allow(dead_code)]
pub fn use_transport_connection(server_url: String) -> Signal<ConnectionState> {
    let mut connection_state = use_signal(|| ConnectionState::Disconnected);
    
    use_effect(move || {
        let url = server_url.clone();
        wasm_bindgen_futures::spawn_local(async move {
            connection_state.set(ConnectionState::Connecting);
            
            let mut transport = Transport::new();
            match transport.connect(&url).await {
                Ok(_) => {
                    connection_state.set(ConnectionState::Connected);
                    tracing::info!("Transport connected to {}", url);
                }
                Err(e) => {
                    tracing::error!("Transport connection failed: {:?}", e);
                    connection_state.set(ConnectionState::Failed);
                }
            }
        });
    });
    
    connection_state
}



/// Hook for Server-Sent Events connection
#[allow(dead_code)]
pub fn use_sse_connection(url: String) -> Signal<SseState> {
    let mut sse_state = use_signal(|| SseState::Disconnected);
    
    use_effect(move || {
        let url = url.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(window) = web_sys::window() {
                if check_sse_support(&window) {
                    sse_state.set(SseState::Connecting);
                    
                    match EventSource::new(&url) {
                        Ok(event_source) => {
                            sse_state.set(SseState::Connected);
                            tracing::info!("SSE connected to {}", url);
                            
                            // Set up event handlers for AI streaming
                            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                                if let Ok(data) = e.data().dyn_into::<js_sys::JsString>() {
                                    tracing::info!("SSE message: {}", data);
                                    // Handle AI streaming response
                                }
                            }) as Box<dyn FnMut(MessageEvent)>);
                            
                            event_source.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                            onmessage.forget();
                        }
                        Err(e) => {
                            tracing::error!("SSE connection failed: {:?}", e);
                            sse_state.set(SseState::Failed);
                        }
                    }
                } else {
                    sse_state.set(SseState::NotSupported);
                }
            }
        });
    });
    
    sse_state
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SseState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
    NotSupported,
}

/// Hook for local storage persistence
#[allow(dead_code)]
pub fn use_local_storage<T>(key: &str, default_value: T) -> (Signal<T>, impl Fn(T)) 
where 
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + 'static
{
    let storage_key = key.to_string();
    let value = use_signal(|| default_value);
    
    // Load from localStorage on first use
    use_effect({
        let key = storage_key.clone();
        let mut val = value.clone();
        move || {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(stored)) = storage.get_item(&key) {
                        if let Ok(parsed) = serde_json::from_str::<T>(&stored) {
                            val.set(parsed);
                        }
                    }
                }
            }
        }
    });
    
    // Create a simple save function that doesn't update the signal
    // (the signal will be updated by the caller)
    let save = {
        let key = storage_key;
        move |new_value: T| {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(serialized) = serde_json::to_string(&new_value) {
                        let _ = storage.set_item(&key, &serialized);
                    }
                }
            }
        }
    };
    
    (value, save)
}

/// Hook for file upload handling
#[allow(dead_code)]
pub fn use_file_upload() -> impl Fn() -> Option<web_sys::File> {
    move || {
        // Create file input element
        let document = web_sys::window()?.document()?;
        let input: web_sys::HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
        
        input.set_type("file");
        input.set_accept(".tex,.latex,.bib,.sty");
        input.click();
        
        // TODO: Return selected file
        // This would typically involve setting up an event handler
        None
    }
}

/// Hook for managing web-specific keyboard shortcuts
#[allow(dead_code)]
pub fn use_web_shortcuts() {
    use_effect(move || {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        
        let keydown_handler = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            // Handle Ctrl+S, Ctrl+O, etc.
            if e.ctrl_key() {
                match e.key().as_str() {
                    "s" => {
                        e.prevent_default();
                        tracing::info!("Save shortcut triggered");
                        // Trigger save
                    }
                    "o" => {
                        e.prevent_default();
                        tracing::info!("Open shortcut triggered");
                        // Trigger open
                    }
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        
        document
            .add_event_listener_with_callback("keydown", keydown_handler.as_ref().unchecked_ref())
            .unwrap();
        
        // Keep closure alive
        keydown_handler.forget();
    });
}