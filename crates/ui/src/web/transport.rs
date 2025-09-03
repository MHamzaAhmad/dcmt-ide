#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use dioxus_signals::{Signal, Readable, Writable};
use serde::{Serialize, Deserialize};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    web_sys::{WebSocket, MessageEvent, BinaryType},
    js_sys::Uint8Array,
    gloo_timers,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    ConnectingWebTransport,
    ConnectingWebSocket,
    Connected,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransportType {
    WebTransport,
    WebSocket,
}

/// Unified transport message types for LaTeX IDE
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransportMessage {
    // File operations
    FileOperation { operation: FileOp },
    
    // LaTeX compilation
    CompilationRequest { document_id: String, content: String, engine: String },
    CompilationResult { document_id: String, success: bool, pdf_data: Option<Vec<u8>>, log: String },
    
    // Git operations  
    GitOperation { operation: GitOp },
    GitResponse { success: bool, data: Option<GitResponseData>, error: Option<String> },
    
    // Health check
    Ping,
    Pong,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

// Placeholder types for now - these will be defined properly when the transport layer is complete
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GitOp {
    // Will be filled in later
    Placeholder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]  
pub enum GitResponseData {
    // Will be filled in later
    Placeholder,
}

/// Unified connection manager that tries WebTransport first, falls back to WebSocket
#[derive(Clone)]
pub struct ConnectionManager {
    server_url: String,
    connection_state: Signal<ConnectionState>,
    transport_type: Signal<Option<TransportType>>,
    #[cfg(target_arch = "wasm32")]
    websocket: Rc<RefCell<Option<WebSocket>>>,
    /// Prevent concurrent connection attempts
    #[cfg(target_arch = "wasm32")]
    connecting: Rc<RefCell<bool>>,
}

impl ConnectionManager {
    pub fn new(server_url: String) -> Self {
        tracing::info!("🔧 Creating ConnectionManager for server: {}", server_url);
        tracing::debug!("📊 ConnectionManager initialization details:");
        tracing::debug!("  └─ Target server URL: {}", server_url);
        tracing::debug!("  └─ Initial state: Disconnected");
        tracing::debug!("  └─ Transport type: None");
        
        #[cfg(target_arch = "wasm32")]
        {
            tracing::debug!("🌐 Platform: WebAssembly (Browser)");
            if let Some(window) = web_sys::window() {
                tracing::debug!("  └─ Window object available");
                // Note: Additional browser info can be added when needed
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            tracing::debug!("🖥️  Platform: Native (Desktop)");
        }
        
        let connection_state = Signal::new(ConnectionState::Disconnected);
        let transport_type = Signal::new(None);
        
        let manager = Self {
            server_url: server_url.clone(),
            connection_state,
            transport_type,
            #[cfg(target_arch = "wasm32")]
            websocket: Rc::new(RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            connecting: Rc::new(RefCell::new(false)),
        };
        
        tracing::info!("✅ ConnectionManager created successfully for {}", server_url);
        manager
    }
    
    pub fn connection_state(&self) -> Signal<ConnectionState> {
        self.connection_state
    }
    
    pub fn transport_type(&self) -> Signal<Option<TransportType>> {
        self.transport_type
    }
    
    pub fn is_connected(&self) -> bool {
        *self.connection_state.read() == ConnectionState::Connected
    }
    
    /// Attempt to connect using WebTransport first, fallback to WebSocket with retry logic
    pub async fn connect_with_retry(&mut self, max_retries: u32) -> Result<(), String> {
        tracing::info!("🔄 Starting connection with retry - max attempts: {}", max_retries);
        let mut attempt = 0;
        let mut last_error = String::new();
        
        while attempt < max_retries {
            tracing::info!("🎯 Connection attempt {}/{} to {}", attempt + 1, max_retries, self.server_url);
            
            match self.connect().await {
                Ok(_) => {
                    tracing::info!("🎉 Connection successful after {} attempts", attempt + 1);
                    return Ok(());
                },
                Err(e) => {
                    last_error = e.clone();
                    attempt += 1;
                    
                    tracing::warn!("❌ Connection attempt {}/{} failed: {}", attempt, max_retries, e);
                    
                    if attempt < max_retries {
                        // Fast retry for initial connection: 100ms, 200ms, 500ms, max 1s
                        let delay_ms = std::cmp::min(100 * (1 << attempt), 1000);
                        tracing::info!("⏳ Waiting {}ms before retry attempt {}/{}", delay_ms, attempt + 1, max_retries);
                        
                        #[cfg(target_arch = "wasm32")]
                        {
                            gloo_timers::future::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        }
                    } else {
                        tracing::error!("💥 All connection attempts exhausted");
                    }
                }
            }
        }
        
        let final_error = format!("Failed to connect after {} attempts. Last error: {}", max_retries, last_error);
        tracing::error!("🚫 {}", final_error);
        Err(final_error)
    }
    
    /// Attempt to connect using WebTransport first, fallback to WebSocket
    pub async fn connect(&mut self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            tracing::debug!("🚀 Starting connection attempt to {}", self.server_url);
            
            // Check if already connecting to prevent concurrent attempts
            let is_connecting = *self.connecting.borrow();
            if is_connecting {
                tracing::warn!("⚠️  Connection already in progress, aborting new attempt");
                return Err("Connection already in progress".to_string());
            }
            
            // Check current connection state
            let current_state = *self.connection_state.read();
            tracing::debug!("📊 Current connection state: {:?}", current_state);
            
            // If already connected, return early
            if self.is_connected() {
                tracing::info!("✅ Already connected, skipping connection attempt");
                return Ok(());
            }
            
            // Mark as connecting
            tracing::debug!("🔒 Marking connection as in progress");
            *self.connecting.borrow_mut() = true;
            
            // Define a guard to reset the flag when function exits
            struct ConnectingGuard(Rc<RefCell<bool>>);
            impl Drop for ConnectingGuard {
                fn drop(&mut self) {
                    *self.0.borrow_mut() = false;
                    tracing::debug!("🔓 Released connection lock");
                }
            }
            let _guard = ConnectingGuard(self.connecting.clone());
            // Try WebTransport first if supported
            let wt_supported = crate::web::hooks::detect_webtransport_support();
            tracing::debug!("🌐 WebTransport support detected: {}", wt_supported);
            
            if wt_supported {
                tracing::info!("🚀 Attempting WebTransport connection to {}", self.server_url);
                
                // Update connection state
                match self.connection_state.try_write() {
                    Ok(mut state) => {
                        *state = ConnectionState::ConnectingWebTransport;
                        tracing::debug!("📊 State updated to: ConnectingWebTransport");
                    },
                    Err(e) => {
                        tracing::warn!("⚠️  Failed to acquire connection state write lock for WebTransport: {:?}", e);
                        // Continue with connection attempt anyway
                    }
                }
                
                let wt_url = if self.server_url.starts_with("http") {
                    self.server_url.replace("http", "https")
                } else {
                    format!("https://{}/webtransport", self.server_url)
                };
                tracing::debug!("🔗 WebTransport URL: {}", wt_url);
                
                match self.try_webtransport(&wt_url).await {
                    Ok(_) => {
                        tracing::debug!("📊 WebTransport connection successful, updating signals");
                        
                        if let Ok(mut transport_type) = self.transport_type.try_write() {
                            *transport_type = Some(TransportType::WebTransport);
                            tracing::debug!("  └─ Transport type set to WebTransport");
                        } else {
                            tracing::warn!("  └─ Failed to set transport type");
                        }
                        
                        if let Ok(mut state) = self.connection_state.try_write() {
                            *state = ConnectionState::Connected;
                            tracing::debug!("  └─ Connection state set to Connected");
                        } else {
                            tracing::warn!("  └─ Failed to set connection state");
                        }
                        
                        tracing::info!("🎉 Connected via WebTransport to {}", self.server_url);
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("❌ WebTransport failed: {}", e);
                        tracing::info!("🔄 Trying WebSocket fallback");
                    }
                }
            } else {
                tracing::info!("⚠️  WebTransport not supported, skipping to WebSocket");
            }
            
            // Fallback to WebSocket
            tracing::info!("🔌 Attempting WebSocket connection to {}", self.server_url);
            
            // Update connection state for WebSocket attempt
            match self.connection_state.try_write() {
                Ok(mut state) => {
                    *state = ConnectionState::ConnectingWebSocket;
                    tracing::debug!("📊 State updated to: ConnectingWebSocket");
                },
                Err(e) => {
                    tracing::warn!("⚠️  Failed to acquire connection state write lock for WebSocket: {:?}", e);
                }
            }
            
            let ws_url = if self.server_url.starts_with("ws") {
                format!("{}/ws", self.server_url)
            } else {
                format!("ws://{}/ws", self.server_url)
            };
            tracing::debug!("🔗 WebSocket URL: {}", ws_url);
            
            match self.try_websocket(&ws_url).await {
                Ok(_) => {
                    tracing::debug!("📊 WebSocket connection successful, updating signals");
                    
                    if let Ok(mut transport_type) = self.transport_type.try_write() {
                        *transport_type = Some(TransportType::WebSocket);
                        tracing::debug!("  └─ Transport type set to WebSocket");
                    } else {
                        tracing::warn!("  └─ Failed to set transport type");
                    }
                    
                    if let Ok(mut state) = self.connection_state.try_write() {
                        *state = ConnectionState::Connected;
                        tracing::debug!("  └─ Connection state set to Connected");
                    } else {
                        tracing::warn!("  └─ Failed to set connection state");
                    }
                    
                    tracing::info!("🎉 Connected via WebSocket to {}", self.server_url);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("❌ WebSocket connection failed: {}", e);
                    
                    // Set failed state
                    match self.connection_state.try_write() {
                        Ok(mut state) => {
                            *state = ConnectionState::Failed;
                            tracing::debug!("📊 State updated to: Failed");
                        },
                        Err(err) => {
                            tracing::warn!("⚠️  Failed to set connection state to Failed: {:?}", err);
                        }
                    }
                    
                    let final_error = format!("Both WebTransport and WebSocket failed. WebSocket error: {}", e);
                    tracing::error!("💥 {}", final_error);
                    Err(final_error)
                }
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Err("Transport connections not available on non-WASM targets".to_string())
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn try_webtransport(&self, _url: &str) -> Result<(), String> {
        // WebTransport implementation would go here
        // For now, we'll skip this and return an error to fallback to WebSocket
        Err("WebTransport implementation pending".to_string())
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn try_websocket(&self, url: &str) -> Result<(), String> {
        tracing::debug!("🔌 Creating WebSocket connection to: {}", url);
        
        let ws = WebSocket::new(url).map_err(|e| {
            let error = format!("Failed to create WebSocket: {:?}", e);
            tracing::error!("❌ {}", error);
            error
        })?;
        
        tracing::debug!("  └─ WebSocket object created successfully");
        ws.set_binary_type(BinaryType::Arraybuffer);
        tracing::debug!("  └─ Binary type set to ArrayBuffer");
        
        let connected = Rc::new(RefCell::new(false));
        let error_msg = Rc::new(RefCell::new(None::<String>));
        
        // Setup connection handlers with detailed logging
        let connected_clone = connected.clone();
        let onopen = Closure::wrap(Box::new(move || {
            *connected_clone.borrow_mut() = true;
            tracing::info!("🎉 WebSocket opened successfully");
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        tracing::debug!("  └─ onopen handler set");
        
        let error_clone = error_msg.clone();
        let onerror = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
            let error_detail = format!("WebSocket connection error: {:?}", e);
            *error_clone.borrow_mut() = Some(error_detail.clone());
            tracing::error!("❌ {}", error_detail);
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        tracing::debug!("  └─ onerror handler set");
        
        // Add close handler for debugging
        let onclose = Closure::wrap(Box::new(move |e: web_sys::CloseEvent| {
            let reason = e.reason();
            let code = e.code();
            tracing::warn!("🔒 WebSocket closed - Code: {}, Reason: {}", code, reason);
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        tracing::debug!("  └─ onclose handler set");
        
        // Wait for connection or timeout with detailed progress logging
        tracing::debug!("⏳ Waiting for WebSocket connection...");
        let mut attempts = 0;
        let max_attempts = 50; // 5 seconds timeout
        let check_interval_ms = 100;
        
        while attempts < max_attempts && !*connected.borrow() && error_msg.borrow().is_none() {
            if attempts % 10 == 0 && attempts > 0 {
                tracing::debug!("⌛ Still waiting for connection... ({}/{} attempts)", attempts, max_attempts);
            }
            
            gloo_timers::future::sleep(std::time::Duration::from_millis(check_interval_ms)).await;
            attempts += 1;
        }
        
        tracing::debug!("⏱️  Connection attempt completed after {} attempts", attempts);
        
        // Clean up closures
        onopen.forget();
        onerror.forget();
        onclose.forget();
        
        // Check results
        if let Some(error) = error_msg.borrow().as_ref() {
            tracing::error!("💥 WebSocket connection failed with error: {}", error);
            return Err(error.clone());
        }
        
        if !*connected.borrow() {
            let timeout_error = format!("Connection timeout after {} attempts", attempts);
            tracing::error!("⏰ {}", timeout_error);
            return Err(timeout_error);
        }
        
        tracing::debug!("💾 Storing WebSocket connection reference");
        *self.websocket.borrow_mut() = Some(ws);
        tracing::info!("✅ WebSocket connection established successfully");
        Ok(())
    }
    
    /// Send a message through the active connection
    pub async fn send_message(&self, _message: TransportMessage) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            match self.transport_type.read().as_ref() {
                Some(TransportType::WebSocket) => {
                    if let Some(ws) = self.websocket.borrow().as_ref() {
                        let data = serde_json::to_vec(&message)
                            .map_err(|e| format!("Failed to serialize message: {}", e))?;
                        
                        let array = Uint8Array::from(&data[..]);
                        ws.send_with_u8_array(&array.to_vec())
                            .map_err(|e| format!("Failed to send WebSocket message: {:?}", e))?;
                        
                        tracing::debug!("Sent message via WebSocket: {:?}", message);
                        Ok(())
                    } else {
                        Err("WebSocket not connected".to_string())
                    }
                }
                Some(TransportType::WebTransport) => {
                    // WebTransport sending would be implemented here
                    Err("WebTransport sending not implemented yet".to_string())
                }
                None => Err("No active connection".to_string()),
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Err("Transport connections not available on non-WASM targets".to_string())
        }
    }
    
    /// Setup message handler for receiving responses
    #[cfg(target_arch = "wasm32")]
    pub fn setup_message_handler<F>(&self, mut handler: F) -> Result<(), String> 
    where
        F: FnMut(TransportMessage) + 'static,
    {
        if let Some(ws) = self.websocket.borrow().as_ref() {
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let uint8_array = Uint8Array::new(&array_buffer);
                    let data = uint8_array.to_vec();
                    
                    match serde_json::from_slice::<TransportMessage>(&data) {
                        Ok(message) => {
                            tracing::debug!("Received message via WebSocket: {:?}", message);
                            handler(message);
                        }
                        Err(e) => {
                            tracing::error!("Failed to deserialize message: {}", e);
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget(); // Keep closure alive
            
            Ok(())
        } else {
            Err("WebSocket not connected".to_string())
        }
    }
    
    pub async fn disconnect(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ws) = self.websocket.borrow_mut().take() {
                let _ = ws.close();
            }
        }
        
        if let Ok(mut state) = self.connection_state.try_write() {
            *state = ConnectionState::Disconnected;
        }
        if let Ok(mut transport_type) = self.transport_type.try_write() {
            *transport_type = None;
        }
        tracing::info!("Disconnected from server");
    }
}

/// Hook to create and manage a connection manager
pub fn use_connection_manager(server_url: String) -> ConnectionManager {
    ConnectionManager::new(server_url)
}