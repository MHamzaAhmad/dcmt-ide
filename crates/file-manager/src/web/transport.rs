use serde::{Serialize, Deserialize};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen::closure::Closure,
    web_sys::{WebSocket, MessageEvent, BinaryType},
    js_sys::Uint8Array,
    gloo_timers,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransportMessage {
    FileOperation { operation: FileOp },
    CompilationRequest { document_id: String, content: String, engine: String },
    CompilationResult { document_id: String, success: bool, pdf_data: Option<Vec<u8>>, log: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

/// WebSocket client for file operations
#[cfg(target_arch = "wasm32")]
pub struct FileTransportClient {
    websocket: Option<WebSocket>,
    connected: bool,
}

#[cfg(target_arch = "wasm32")]
impl FileTransportClient {
    pub fn new() -> Self {
        Self {
            websocket: None,
            connected: false,
        }
    }
    
    pub async fn connect(&mut self, _url: &str) -> Result<(), String> {
        let ws_url = "ws://localhost:3001/ws";
        
        match WebSocket::new(ws_url) {
            Ok(ws) => {
                ws.set_binary_type(BinaryType::Arraybuffer);
                
                // Wait for connection to open
                let connected_ref = std::rc::Rc::new(std::cell::RefCell::new(false));
                let error_ref = std::rc::Rc::new(std::cell::RefCell::new(None::<String>));
                
                // Setup onopen handler
                let connected_clone = connected_ref.clone();
                let onopen = Closure::wrap(Box::new(move || {
                    *connected_clone.borrow_mut() = true;
                    tracing::info!("WebSocket connected to file server");
                }) as Box<dyn FnMut()>);
                ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                
                // Setup onerror handler
                let error_clone = error_ref.clone();
                let onerror = Closure::wrap(Box::new(move |_e| {
                    *error_clone.borrow_mut() = Some("Connection error".to_string());
                    tracing::error!("WebSocket connection error");
                }) as Box<dyn FnMut(web_sys::Event)>);
                ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                
                // Wait for connection
                let mut attempts = 0;
                while attempts < 50 {
                    if *connected_ref.borrow() {
                        break;
                    }
                    if let Some(error) = error_ref.borrow().as_ref() {
                        return Err(error.clone());
                    }
                    gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }
                
                if *connected_ref.borrow() {
                    self.websocket = Some(ws);
                    self.connected = true;
                    
                    // Prevent closures from being dropped
                    onopen.forget();
                    onerror.forget();
                    
                    Ok(())
                } else {
                    Err("Connection timeout".to_string())
                }
            }
            Err(e) => Err(format!("Failed to create WebSocket: {:?}", e))
        }
    }
    
    pub async fn send_operation(&self, operation: FileOp) -> Result<TransportMessage, String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }
        
        if let Some(ws) = &self.websocket {
            let message = TransportMessage::FileOperation { operation };
            let serialized = serde_json::to_vec(&message)
                .map_err(|e| format!("Serialization error: {}", e))?;
                
            let array = js_sys::Uint8Array::from(&serialized[..]);
            ws.send_with_array_buffer(&array.buffer())
                .map_err(|e| format!("Send error: {:?}", e))?;
            
            // Set up response handler
            let response = std::rc::Rc::new(std::cell::RefCell::new(None::<Result<TransportMessage, String>>));
            let response_clone = response.clone();
            
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let uint8_array = Uint8Array::new(&array_buffer);
                    let data = uint8_array.to_vec();
                    
                    match serde_json::from_slice::<TransportMessage>(&data) {
                        Ok(msg) => {
                            *response_clone.borrow_mut() = Some(Ok(msg));
                        }
                        Err(e) => {
                            *response_clone.borrow_mut() = Some(Err(format!("Deserialization error: {}", e)));
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            
            // Wait for response
            let mut attempts = 0;
            while attempts < 100 {
                if let Some(result) = response.borrow().as_ref() {
                    let result = result.clone();
                    onmessage.forget();
                    return result;
                }
                gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;
                attempts += 1;
            }
            
            onmessage.forget();
            Err("Response timeout".to_string())
        } else {
            Err("WebSocket not available".to_string())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct FileTransportClient;

#[cfg(not(target_arch = "wasm32"))]
impl FileTransportClient {
    pub fn new() -> Self { Self }
    
    pub async fn connect(&mut self, _url: &str) -> Result<(), String> {
        Err("Transport not available on non-WASM".to_string())
    }
    
    pub async fn send_operation(&self, _operation: FileOp) -> Result<TransportMessage, String> {
        Err("Transport not available on non-WASM".to_string())
    }
}

/// High-level file operations using transport client
pub async fn upload_file(name: &str, content: Vec<u8>) -> Result<(), String> {
    let mut client = FileTransportClient::new();
    client.connect("ws://localhost:3001").await?;
    
    let operation = FileOp::Upload {
        name: name.to_string(),
        content,
    };
    
    match client.send_operation(operation).await? {
        TransportMessage::FileOperation { operation: FileOp::Upload { name, .. } } => {
            if name.starts_with("Error:") {
                Err(name)
            } else {
                Ok(())
            }
        }
        _ => Err("Unexpected response".to_string())
    }
}

pub async fn download_file(name: &str) -> Result<Vec<u8>, String> {
    let mut client = FileTransportClient::new();
    client.connect("ws://localhost:3001").await?;
    
    let operation = FileOp::Download {
        name: name.to_string(),
    };
    
    match client.send_operation(operation).await? {
        TransportMessage::FileOperation { operation: FileOp::Upload { name, content } } => {
            if name.starts_with("Error:") {
                Err(name)
            } else {
                // Server returns actual file content as bytes
                tracing::info!("Downloaded file content: {} bytes", content.len());
                Ok(content)
            }
        }
        _ => Err("Unexpected response from server".to_string())
    }
}

pub async fn delete_file(name: &str) -> Result<(), String> {
    let mut client = FileTransportClient::new();
    client.connect("ws://localhost:3001").await?;
    
    let operation = FileOp::Delete {
        name: name.to_string(),
    };
    
    match client.send_operation(operation).await? {
        TransportMessage::FileOperation { operation: FileOp::Delete { name } } => {
            if name.starts_with("Error:") {
                Err(name)
            } else {
                Ok(())
            }
        }
        _ => Err("Unexpected response".to_string())
    }
}

pub async fn list_files() -> Result<Vec<FileInfo>, String> {
    let mut client = FileTransportClient::new();
    client.connect("ws://localhost:3001").await?;
    
    let operation = FileOp::List;
    
    match client.send_operation(operation).await? {
        TransportMessage::FileOperation { operation: FileOp::Download { name } } => {
            // Server returns file list as JSON in the name field
            if name.starts_with("Error:") {
                Err(name)
            } else {
                // Parse JSON file list
                match serde_json::from_str::<Vec<FileInfo>>(&name) {
                    Ok(files) => {
                        tracing::info!("Successfully parsed {} files from server", files.len());
                        Ok(files)
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse file list JSON: {}", e);
                        Err(format!("Failed to parse file list: {}", e))
                    }
                }
            }
        }
        _ => Err("Unexpected response from server".to_string())
    }
}