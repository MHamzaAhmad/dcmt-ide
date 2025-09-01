use wasm_bindgen::prelude::*;
use web_sys::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// WebTransport client for collaborative editing
pub struct WebTransportClient {
    transport: Option<web_sys::WebTransport>,
    streams: Vec<web_sys::WebTransportBidirectionalStream>,
    connection_state: ConnectionState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WebTransportMessage {
    // Document synchronization (Stream 0)
    YrsUpdate {
        document_id: String,
        update: Vec<u8>,
    },
    
    // Awareness updates (Stream 1)
    Awareness {
        user_id: String,
        cursor_position: Option<u32>,
        selection_range: Option<(u32, u32)>,
    },
    
    // AI chat (Stream 2)
    AiMessage {
        conversation_id: String,
        message: String,
        model_id: String,
    },
    
    // File operations (Stream 3)
    FileOperation {
        operation: FileOp,
    },
    
    // Compilation requests (Stream 4)
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

#[derive(Serialize, Deserialize, Debug)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

impl WebTransportClient {
    pub fn new() -> Self {
        Self {
            transport: None,
            streams: Vec::new(),
            connection_state: ConnectionState::Disconnected,
        }
    }
    
    pub async fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        self.connection_state = ConnectionState::Connecting;
        
        // Check if WebTransport is supported
        let window = web_sys::window().ok_or("No window")?;
        if !js_sys::Reflect::has(&window, &"WebTransport".into()).unwrap_or(false) {
            return Err("WebTransport not supported".into());
        }
        
        // Create WebTransport instance
        let transport = web_sys::WebTransport::new(url)?;
        
        // Wait for connection to be ready
        let ready = transport.ready();
        wasm_bindgen_futures::JsFuture::from(ready).await?;
        
        self.transport = Some(transport);
        self.connection_state = ConnectionState::Connected;
        
        // Set up multiplexed streams
        self.setup_streams().await?;
        
        tracing::info!("WebTransport connected to {}", url);
        Ok(())
    }
    
    async fn setup_streams(&mut self) -> Result<(), JsValue> {
        if let Some(transport) = &self.transport {
            // Create 5 bidirectional streams for different purposes
            for stream_id in 0..5 {
                let stream = transport.create_bidirectional_stream().await?;
                self.streams.push(stream);
                
                tracing::debug!("Created WebTransport stream {}", stream_id);
            }
        }
        Ok(())
    }
    
    pub async fn send_message(&self, stream_id: usize, message: WebTransportMessage) -> Result<(), JsValue> {
        if let Some(stream) = self.streams.get(stream_id) {
            let serialized = serde_json::to_vec(&message)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
            let writable = stream.writable();
            let writer = writable.get_writer()?;
            
            // Write message length first (4 bytes)
            let length_bytes = (serialized.len() as u32).to_be_bytes();
            let length_array = js_sys::Uint8Array::from(&length_bytes[..]);
            let length_promise = writer.write_with_chunk(&length_array);
            wasm_bindgen_futures::JsFuture::from(length_promise).await?;
            
            // Write message data
            let data_array = js_sys::Uint8Array::from(&serialized[..]);
            let data_promise = writer.write_with_chunk(&data_array);
            wasm_bindgen_futures::JsFuture::from(data_promise).await?;
            
            writer.release_lock();
        }
        
        Ok(())
    }
    
    pub async fn receive_message(&self, stream_id: usize) -> Result<Option<WebTransportMessage>, JsValue> {
        if let Some(stream) = self.streams.get(stream_id) {
            let readable = stream.readable();
            let reader = readable.get_reader()?;
            
            // Read message length (4 bytes)
            let length_result = reader.read().await?;
            let length_value = js_sys::Reflect::get(&length_result, &"value".into())?;
            
            if length_value.is_undefined() {
                return Ok(None); // Stream ended
            }
            
            let length_array: js_sys::Uint8Array = length_value.dyn_into()?;
            let length_bytes = length_array.to_vec();
            
            if length_bytes.len() != 4 {
                return Err("Invalid message length".into());
            }
            
            let message_length = u32::from_be_bytes([
                length_bytes[0], 
                length_bytes[1], 
                length_bytes[2], 
                length_bytes[3]
            ]) as usize;
            
            // Read message data
            let data_result = reader.read().await?;
            let data_value = js_sys::Reflect::get(&data_result, &"value".into())?;
            let data_array: js_sys::Uint8Array = data_value.dyn_into()?;
            let message_bytes = data_array.to_vec();
            
            reader.release_lock();
            
            if message_bytes.len() != message_length {
                return Err("Message length mismatch".into());
            }
            
            let message: WebTransportMessage = serde_json::from_slice(&message_bytes)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
    
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }
    
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
    
    pub async fn disconnect(&mut self) {
        if let Some(transport) = &self.transport {
            transport.close();
            self.transport = None;
            self.streams.clear();
            self.connection_state = ConnectionState::Disconnected;
            
            tracing::info!("WebTransport disconnected");
        }
    }
}

/// WebSocket fallback for browsers without WebTransport support
pub struct WebSocketFallback {
    websocket: Option<WebSocket>,
    connection_state: ConnectionState,
}

impl WebSocketFallback {
    pub fn new() -> Self {
        Self {
            websocket: None,
            connection_state: ConnectionState::Disconnected,
        }
    }
    
    pub async fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        self.connection_state = ConnectionState::Connecting;
        
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        
        // Set up event handlers
        let onopen = Closure::wrap(Box::new({
            let mut state = self.connection_state.clone();
            move || {
                state = ConnectionState::Connected;
                tracing::info!("WebSocket connected");
            }
        }) as Box<dyn FnMut()>);
        
        let onerror = Closure::wrap(Box::new({
            let mut state = self.connection_state.clone();
            move |e: ErrorEvent| {
                state = ConnectionState::Failed;
                tracing::error!("WebSocket error: {:?}", e);
            }
        }) as Box<dyn FnMut(ErrorEvent)>);
        
        let onclose = Closure::wrap(Box::new({
            let mut state = self.connection_state.clone();
            move |e: CloseEvent| {
                state = ConnectionState::Disconnected;
                tracing::info!("WebSocket closed: {}", e.reason());
            }
        }) as Box<dyn FnMut(CloseEvent)>);
        
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        
        // Keep closures alive
        onopen.forget();
        onerror.forget();
        onclose.forget();
        
        self.websocket = Some(ws);
        Ok(())
    }
    
    pub fn send_message(&self, message: WebTransportMessage) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            let serialized = serde_json::to_vec(&message)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            
            let array = js_sys::Uint8Array::from(&serialized[..]);
            ws.send_with_u8_array(&array)?;
        }
        
        Ok(())
    }
    
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }
    
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.websocket {
            ws.close().ok();
            self.websocket = None;
            self.connection_state = ConnectionState::Disconnected;
        }
    }
}

/// Unified connection manager that tries WebTransport first, falls back to WebSocket
pub struct ConnectionManager {
    webtransport: Option<WebTransportClient>,
    websocket: Option<WebSocketFallback>,
    current_connection: ConnectionType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionType {
    None,
    WebTransport,
    WebSocket,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            webtransport: None,
            websocket: None,
            current_connection: ConnectionType::None,
        }
    }
    
    pub async fn connect(&mut self, base_url: &str) -> Result<(), JsValue> {
        // Try WebTransport first
        let wt_url = format!("https://{}/webtransport", base_url);
        let mut wt_client = WebTransportClient::new();
        
        match wt_client.connect(&wt_url).await {
            Ok(_) => {
                self.webtransport = Some(wt_client);
                self.current_connection = ConnectionType::WebTransport;
                tracing::info!("Using WebTransport connection");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("WebTransport failed, trying WebSocket fallback: {:?}", e);
            }
        }
        
        // Fallback to WebSocket
        let ws_url = format!("wss://{}/ws", base_url);
        let mut ws_client = WebSocketFallback::new();
        
        match ws_client.connect(&ws_url).await {
            Ok(_) => {
                self.websocket = Some(ws_client);
                self.current_connection = ConnectionType::WebSocket;
                tracing::info!("Using WebSocket fallback connection");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Both WebTransport and WebSocket failed: {:?}", e);
                Err(e)
            }
        }
    }
    
    pub async fn send_message(&self, message: WebTransportMessage) -> Result<(), JsValue> {
        match &self.current_connection {
            ConnectionType::WebTransport => {
                if let Some(wt) = &self.webtransport {
                    // Determine stream based on message type
                    let stream_id = match &message {
                        WebTransportMessage::YrsUpdate { .. } => 0,
                        WebTransportMessage::Awareness { .. } => 1,
                        WebTransportMessage::AiMessage { .. } => 2,
                        WebTransportMessage::FileOperation { .. } => 3,
                        WebTransportMessage::CompilationRequest { .. } |
                        WebTransportMessage::CompilationResult { .. } => 4,
                    };
                    
                    wt.send_message(stream_id, message).await
                } else {
                    Err("WebTransport not available".into())
                }
            }
            ConnectionType::WebSocket => {
                if let Some(ws) = &self.websocket {
                    ws.send_message(message)
                } else {
                    Err("WebSocket not available".into())
                }
            }
            ConnectionType::None => Err("No connection available".into()),
        }
    }
    
    pub fn connection_type(&self) -> &ConnectionType {
        &self.current_connection
    }
    
    pub fn is_connected(&self) -> bool {
        match &self.current_connection {
            ConnectionType::WebTransport => {
                self.webtransport.as_ref().map_or(false, |wt| wt.is_connected())
            }
            ConnectionType::WebSocket => {
                self.websocket.as_ref().map_or(false, |ws| {
                    ws.connection_state() == &ConnectionState::Connected
                })
            }
            ConnectionType::None => false,
        }
    }
    
    pub async fn disconnect(&mut self) {
        if let Some(wt) = &mut self.webtransport {
            wt.disconnect().await;
        }
        if let Some(ws) = &mut self.websocket {
            ws.disconnect();
        }
        
        self.current_connection = ConnectionType::None;
    }
}