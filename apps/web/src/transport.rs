use serde::{Serialize, Deserialize};

// For native builds, use wtransport
#[cfg(not(target_arch = "wasm32"))]
use {
    wtransport::{Connection, Endpoint, ClientConfig},
    tokio::sync::Mutex,
    std::collections::HashMap,
    tokio::io::{AsyncReadExt, AsyncWriteExt},
    std::sync::Arc,
};

// For WASM builds, use WebSocket
#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen::closure::Closure,
    web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent, BinaryType},
    js_sys::Uint8Array,
    std::collections::HashMap,
    std::rc::Rc,
    std::cell::RefCell,
};

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransportMessage {
    // Document synchronization
    YrsUpdate {
        document_id: String,
        update: Vec<u8>,
    },
    
    // Awareness updates
    Awareness {
        user_id: String,
        cursor_position: Option<u32>,
        selection_range: Option<(u32, u32)>,
    },
    
    // AI chat
    AiMessage {
        conversation_id: String,
        message: String,
        model_id: String,
    },
    
    // File operations
    FileOperation {
        operation: FileOp,
    },
    
    // Compilation requests
    CompilationRequest {
        document_id: String,
        content: String,
        engine: String,
    },
    
    CompilationResult {
        document_id: String,
        success: bool,
        pdf_data: Option<Vec<u8>>,
        log: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

// Native WebTransport implementation using wtransport
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub struct Transport {
    connection: Option<Connection>,
    connection_state: ConnectionState,
    message_handlers: Arc<Mutex<HashMap<String, Arc<dyn Fn(TransportMessage) + Send + Sync>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
impl Transport {
    pub fn new() -> Self {
        Self {
            connection: None,
            connection_state: ConnectionState::Disconnected,
            message_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connection_state = ConnectionState::Connecting;
        
        // Configure wtransport client
        let config = ClientConfig::builder()
            .with_bind_default()
            .with_no_cert_validation()
            .build();
            
        let endpoint = Endpoint::client(config)?;
        let connection = endpoint.connect(url.parse()?).await?;
            
        self.connection = Some(connection);
        self.connection_state = ConnectionState::Connected;
        
        tracing::info!("WebTransport connected to {}", url);
        Ok(())
    }
    
    pub async fn send_message(&self, message: TransportMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(connection) = &self.connection {
            let (mut send_stream, _recv_stream) = connection.open_bi().await?;
            let serialized = serde_json::to_vec(&message)?;
            
            // Write message length first (4 bytes)
            let length = serialized.len() as u32;
            send_stream.write_all(&length.to_be_bytes()).await?;
            
            // Write message data
            send_stream.write_all(&serialized).await?;
            send_stream.finish().await?;
            
            Ok(())
        } else {
            Err("No WebTransport connection".into())
        }
    }
    
    pub async fn on_message<F>(&self, message_type: &str, handler: F)
    where
        F: Fn(TransportMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.lock().await;
        handlers.insert(message_type.to_string(), Arc::new(handler));
    }
    
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }
    
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
    
    pub async fn disconnect(&mut self) {
        if let Some(connection) = &self.connection {
            connection.close(0u32.into(), b"closing");
            self.connection = None;
            self.connection_state = ConnectionState::Disconnected;
            tracing::info!("WebTransport disconnected");
        }
    }
}

// WASM WebSocket implementation
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub struct Transport {
    websocket: Option<WebSocket>,
    connection_state: ConnectionState,
    message_handlers: HashMap<String, Rc<dyn Fn(TransportMessage)>>,
    _closures: Vec<Box<dyn AsRef<JsValue>>>,
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
impl Transport {
    pub fn new() -> Self {
        Self {
            websocket: None,
            connection_state: ConnectionState::Disconnected,
            message_handlers: HashMap::new(),
            _closures: Vec::new(),
        }
    }
    
    pub async fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        self.connection_state = ConnectionState::Connecting;
        
        // Convert to WebSocket URL
        let ws_url = if url.starts_with("wss://") || url.starts_with("ws://") {
            url.to_string()
        } else if url.starts_with("https://") {
            url.replace("https://", "wss://")
        } else {
            format!("wss://{}/ws", url)
        };
        
        let ws = WebSocket::new(&ws_url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        
        let state = Rc::new(RefCell::new(ConnectionState::Connecting));
        let handlers = Rc::new(RefCell::new(HashMap::<String, Rc<dyn Fn(TransportMessage)>>::new()));
        
        // Connection opened
        let state_clone = state.clone();
        let onopen = Closure::wrap(Box::new(move || {
            *state_clone.borrow_mut() = ConnectionState::Connected;
            tracing::info!("WebSocket connected");
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        
        // Connection error
        let state_clone = state.clone();
        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            *state_clone.borrow_mut() = ConnectionState::Failed;
            tracing::error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        
        // Connection closed
        let state_clone = state.clone();
        let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
            *state_clone.borrow_mut() = ConnectionState::Disconnected;
            tracing::info!("WebSocket closed: {}", e.reason());
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        
        // Message received
        let handlers_clone = handlers.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = Uint8Array::new(&array_buffer);
                let data = uint8_array.to_vec();
                
                match serde_json::from_slice::<TransportMessage>(&data) {
                    Ok(message) => {
                        let message_type = match &message {
                            TransportMessage::YrsUpdate { .. } => "yrs_update",
                            TransportMessage::Awareness { .. } => "awareness",
                            TransportMessage::AiMessage { .. } => "ai_message",
                            TransportMessage::FileOperation { .. } => "file_operation",
                            TransportMessage::CompilationRequest { .. } => "compilation_request",
                            TransportMessage::CompilationResult { .. } => "compilation_result",
                        };
                        
                        if let Some(handler) = handlers_clone.borrow().get(message_type) {
                            handler(message.clone());
                        }
                        
                        if let Some(global_handler) = handlers_clone.borrow().get("*") {
                            global_handler(message);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize message: {:?}", e);
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        
        self.websocket = Some(ws);
        
        // Store closures to prevent them from being dropped
        self._closures.push(Box::new(onopen));
        self._closures.push(Box::new(onclose));
        self._closures.push(Box::new(onerror));
        self._closures.push(Box::new(onmessage));
        
        // Wait for connection
        let mut attempts = 0;
        while attempts < 50 && *state.borrow() == ConnectionState::Connecting {
            gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
            attempts += 1;
        }
        
        self.connection_state = state.borrow().clone();
        
        match self.connection_state {
            ConnectionState::Connected => Ok(()),
            ConnectionState::Failed => Err("Connection failed".into()),
            _ => Err("Connection timeout".into()),
        }
    }
    
    pub async fn send_message(&self, message: TransportMessage) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            if ws.ready_state() == WebSocket::OPEN {
                let serialized = serde_json::to_vec(&message)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                
                let array = Uint8Array::from(&serialized[..]);
                ws.send_with_array_buffer(&array.buffer())?;
                
                Ok(())
            } else {
                Err("WebSocket not connected".into())
            }
        } else {
            Err("No WebSocket connection".into())
        }
    }
    
    pub fn on_message<F>(&mut self, message_type: &str, handler: F)
    where
        F: Fn(TransportMessage) + 'static,
    {
        self.message_handlers.insert(
            message_type.to_string(),
            Rc::new(handler)
        );
    }
    
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }
    
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
    
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.websocket {
            let _ = ws.close();
            self.websocket = None;
            self.connection_state = ConnectionState::Disconnected;
            tracing::info!("WebSocket disconnected");
        }
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(connection) = &self.connection {
                connection.close(0u32.into(), b"dropping");
            }
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            self.disconnect();
        }
    }
}