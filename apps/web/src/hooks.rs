use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::*;
use ywasm::*;

/// Browser capabilities detection
#[derive(Clone, Debug)]
pub struct BrowserCapabilities {
    pub webtransport_supported: bool,
    pub websocket_supported: bool,
    pub server_sent_events: bool,
    pub web_assembly: bool,
    pub local_storage: bool,
    pub indexed_db: bool,
}

/// Hook to detect browser capabilities
pub fn use_browser_capabilities() -> Signal<BrowserCapabilities> {
    let capabilities = use_signal(|| BrowserCapabilities {
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

fn check_webtransport_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"WebTransport".into()).unwrap_or(false)
}

fn check_websocket_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"WebSocket".into()).unwrap_or(false)
}

fn check_sse_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"EventSource".into()).unwrap_or(false)
}

fn check_local_storage_support(window: &Window) -> bool {
    window.local_storage().is_ok()
}

fn check_indexeddb_support(window: &Window) -> bool {
    js_sys::Reflect::has(window, &"indexedDB".into()).unwrap_or(false)
}

/// Hook for ywasm document management
pub fn use_ywasm_document() -> Signal<Option<YDoc>> {
    let doc = use_signal(|| None);
    
    use_effect(move || {
        // Initialize ywasm document
        let ydoc = YDoc::new();
        
        // Get or create text field for document content
        let text = ydoc.get_text("content");
        
        // Set initial content
        text.insert(0, include_str!("../assets/sample.tex"));
        
        doc.set(Some(ydoc));
    });
    
    doc
}

/// Hook for WebTransport connection management
pub fn use_webtransport_connection(server_url: String) -> Signal<ConnectionState> {
    let connection_state = use_signal(|| ConnectionState::Disconnected);
    
    use_effect(move || {
        let url = server_url.clone();
        wasm_bindgen_futures::spawn_local(async move {
            connection_state.set(ConnectionState::Connecting);
            
            match connect_webtransport(&url).await {
                Ok(_) => {
                    connection_state.set(ConnectionState::Connected);
                    tracing::info!("WebTransport connected to {}", url);
                }
                Err(e) => {
                    tracing::error!("WebTransport connection failed: {:?}", e);
                    connection_state.set(ConnectionState::Failed);
                    
                    // Try WebSocket fallback
                    match connect_websocket(&url.replace("https://", "wss://")).await {
                        Ok(_) => {
                            connection_state.set(ConnectionState::FallbackConnected);
                            tracing::info!("WebSocket fallback connected");
                        }
                        Err(e) => {
                            tracing::error!("WebSocket fallback failed: {:?}", e);
                            connection_state.set(ConnectionState::Failed);
                        }
                    }
                }
            }
        });
    });
    
    connection_state
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    FallbackConnected,
    Failed,
}

async fn connect_webtransport(url: &str) -> Result<(), JsValue> {
    // Check if WebTransport is supported
    let window = web_sys::window().ok_or("No window")?;
    
    if !check_webtransport_support(&window) {
        return Err("WebTransport not supported".into());
    }
    
    // TODO: Implement actual WebTransport connection
    // This is a placeholder - actual implementation would use the WebTransport API
    tracing::info!("Attempting WebTransport connection to {}", url);
    
    // Simulate connection attempt
    gloo_timers::future::sleep(std::time::Duration::from_millis(1000)).await;
    
    // For now, always fail to test fallback
    Err("WebTransport connection simulated failure".into())
}

async fn connect_websocket(url: &str) -> Result<(), JsValue> {
    let ws = WebSocket::new(url)?;
    
    // Set up event handlers
    let onopen = Closure::wrap(Box::new(move || {
        tracing::info!("WebSocket connection opened");
    }) as Box<dyn FnMut()>);
    
    let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
        tracing::error!("WebSocket error: {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    
    // Keep closures alive
    onopen.forget();
    onerror.forget();
    
    Ok(())
}

/// Hook for Server-Sent Events connection
pub fn use_sse_connection(url: String) -> Signal<SseState> {
    let sse_state = use_signal(|| SseState::Disconnected);
    
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
pub enum SseState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
    NotSupported,
}

/// Hook for local storage persistence
pub fn use_local_storage<T>(key: &str, default_value: T) -> (Signal<T>, impl Fn(T)) 
where 
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + 'static
{
    let storage_key = key.to_string();
    let value = use_signal(|| default_value);
    
    // Load from localStorage on first use
    use_effect({
        let key = storage_key.clone();
        move || {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(stored)) = storage.get_item(&key) {
                        if let Ok(parsed) = serde_json::from_str::<T>(&stored) {
                            value.set(parsed);
                        }
                    }
                }
            }
        }
    });
    
    // Save function
    let save = {
        let key = storage_key;
        move |new_value: T| {
            value.set(new_value.clone());
            
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
pub fn use_web_shortcuts() {
    use_effect(move || {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        
        let keydown_handler = Closure::wrap(Box::new(move |e: KeyboardEvent| {
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