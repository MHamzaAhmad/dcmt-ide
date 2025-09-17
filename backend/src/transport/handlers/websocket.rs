use crate::transport::routes::websocket::WebSocketServices;
use crate::model::events::{FileEvent, CompilationEvent, EventEnvelope};
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tokio::sync::broadcast;
use uuid::Uuid;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(services): State<WebSocketServices>,
) -> Response {
    info!("WebSocket connection requested");
    ws.on_upgrade(move |socket| handle_websocket(socket, services))
}

async fn handle_websocket(socket: WebSocket, services: WebSocketServices) {
    info!("WebSocket connection established");
    
    let connection_id = Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();
    
    // Create connection state
    let mut connection_state = ConnectionState::new(connection_id.clone(), services);
    
    // Initialize the connection
    if let Err(e) = connection_state.initialize().await {
        error!("Failed to initialize WebSocket connection {}: {}", connection_id, e);
        return;
    }
    
    // Defer confirmation; we'll send a Ready after Identify

    // Handle Identify synchronously before spawning tasks
    let state_for_recv = connection_state.clone();
    if let Some(Ok(Message::Text(text))) = receiver.next().await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            if val.get("type").and_then(|t| t.as_str()) == Some("identify") {
                let last_seen_compilation = val
                    .get("lastSeen")
                    .and_then(|ls| ls.get("compilation"))
                    .and_then(|c| c.as_u64());
                let snapshot = state_for_recv.services.latex_service.get_snapshot().await;
                let current_seq = state_for_recv.services.latex_service.current_compilation_seq();
                let ready = serde_json::json!({
                    "type": "ready",
                    "snapshot": { "latex": snapshot },
                    "cursors": { "compilation": current_seq }
                });
                let _ = state_for_recv.send_json(&mut sender, ready).await;
                if let Some(since) = last_seen_compilation {
                    let events = state_for_recv.services.latex_service.get_compilation_since(since).await;
                    for env in events {
                        let msg = serde_json::json!({
                            "type": "event",
                            "topic": env.topic,
                            "seq": env.seq,
                            "ts": env.ts,
                            "payload": {
                                "event_type": match env.payload.event_type {
                                    crate::model::events::CompilationEventType::Queued => "queued",
                                    crate::model::events::CompilationEventType::Started => "started",
                                    crate::model::events::CompilationEventType::Success => "success",
                                    crate::model::events::CompilationEventType::Error => "error",
                                    crate::model::events::CompilationEventType::MainFileDetected => "main_file_detected",
                                },
                                "main_file": env.payload.main_file,
                                "timestamp": env.payload.timestamp,
                                "metadata": env.payload.metadata
                            }
                        });
                        let _ = state_for_recv.send_json(&mut sender, msg).await;
                    }
                }
            }
        }
    }

    // Now spawn the outgoing sender task
    let send_task = {
        let mut state = connection_state.clone();
        let mut sender_clone = sender;
        let conn_id = connection_id.clone();
        tokio::spawn(async move {
            if let Err(e) = state.handle_outgoing_events(&mut sender_clone).await {
                error!("Event sender task failed for connection {}: {}", conn_id, e);
            }
            debug!("Event sender task ended for connection {}", conn_id);
        })
    };

    let receive_task = {
        let mut state = state_for_recv.clone();
        let conn_id = connection_id.clone();
        let mut rx = receiver;
        tokio::spawn(async move {
            while let Some(msg) = rx.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = state.handle_incoming_message(text.to_string()).await {
                            error!("Failed to handle incoming message for connection {}: {}", conn_id, e);
                        }
                    }
                    Ok(Message::Binary(_)) => {
                        warn!("Received binary message on connection {}, ignoring", conn_id);
                    }
                    Ok(Message::Ping(_)) => {
                        debug!("Received ping on connection {}", conn_id);
                    }
                    Ok(Message::Pong(_)) => {
                        debug!("Received pong on connection {}", conn_id);
                    }
                    Ok(Message::Close(close_frame)) => {
                        info!("WebSocket connection {} closed: {:?}", conn_id, close_frame);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error on connection {}: {}", conn_id, e);
                        break;
                    }
                }
            }
            debug!("Message receiver task ended for connection {}", conn_id);
        })
    };

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            debug!("Send task completed for connection {}", connection_id);
        }
        _ = receive_task => {
            debug!("Receive task completed for connection {}", connection_id);
        }
    }
    
    // Cleanup connection state
    if let Err(e) = connection_state.cleanup().await {
        error!("Failed to cleanup connection {}: {}", connection_id, e);
    }
    
    info!("WebSocket connection handler finished for connection {}", connection_id);
}

/// Production-ready WebSocket connection state management
#[derive(Clone)]
struct ConnectionState {
    connection_id: String,
    services: WebSocketServices,
    file_subscribed: Arc<tokio::sync::RwLock<bool>>,
    compilation_subscribed: Arc<tokio::sync::RwLock<bool>>,
    file_event_receiver: Arc<tokio::sync::Mutex<Option<broadcast::Receiver<FileEvent>>>>,
    compilation_event_receiver: Arc<tokio::sync::Mutex<Option<broadcast::Receiver<CompilationEvent>>>>,
    compilation_env_receiver: Arc<tokio::sync::Mutex<Option<tokio::sync::broadcast::Receiver<EventEnvelope<CompilationEvent>>>>>,
    event_tx: Arc<tokio::sync::mpsc::UnboundedSender<WebSocketEvent>>,
    event_rx: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<WebSocketEvent>>>>,
}

#[derive(Debug, Clone)]
enum WebSocketEvent {
    FileEvent(FileEvent),
    CompilationEnvelope(EventEnvelope<CompilationEvent>),
    ConnectionStatus {
        status: String,
        timestamp: u64,
    },
}

impl ConnectionState {
    fn new(connection_id: String, services: WebSocketServices) -> Self {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        
        Self {
            connection_id,
            services,
            file_subscribed: Arc::new(tokio::sync::RwLock::new(false)),
            compilation_subscribed: Arc::new(tokio::sync::RwLock::new(false)),
            file_event_receiver: Arc::new(tokio::sync::Mutex::new(None)),
            compilation_event_receiver: Arc::new(tokio::sync::Mutex::new(None)),
            compilation_env_receiver: Arc::new(tokio::sync::Mutex::new(None)),
            event_tx: Arc::new(event_tx),
            event_rx: Arc::new(tokio::sync::Mutex::new(Some(event_rx))),
        }
    }
    
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send initial connection confirmation
        let connection_event = WebSocketEvent::ConnectionStatus {
            status: "connected".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        self.event_tx.send(connection_event)
            .map_err(|e| format!("Failed to send connection event: {}", e))?;
        
        info!("Initialized WebSocket connection: {}", self.connection_id);
        Ok(())
    }

    async fn send_json(&self, sender: &mut futures_util::stream::SplitSink<WebSocket, Message>, value: serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let text = value.to_string();
        sender.send(Message::Text(text.into())).await?;
        Ok(())
    }
    
    async fn handle_incoming_message(&mut self, text: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Connection {} received message: {}", self.connection_id, text);
        
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let msg_type = json.get("type")
            .and_then(|t| t.as_str())
            .ok_or("Message missing 'type' field")?;
        
        match msg_type {
            "ping" => {
                debug!("Connection {} received ping", self.connection_id);
                // Pong is handled automatically by axum
            }
            "subscribe_files" => {
                self.subscribe_to_files().await?;
            }
            "unsubscribe_files" => {
                self.unsubscribe_from_files().await?;
            }
            "subscribe_compilation" => {
                self.subscribe_to_compilation().await?;
            }
            "unsubscribe_compilation" => {
                self.unsubscribe_from_compilation().await?;
            }
            // Agent events are now handled via SSE, no longer supported via WebSocket
            "subscribe" => {
                // Legacy support for file events
                self.subscribe_to_files().await?;
            }
            _ => {
                warn!("Connection {} received unknown message type: {}", self.connection_id, msg_type);
            }
        }
        
        Ok(())
    }
    
    async fn subscribe_to_files(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut file_subscribed = self.file_subscribed.write().await;
        
        if *file_subscribed {
            debug!("Connection {} already subscribed to file events", self.connection_id);
            return Ok(());
        }
        
        // Subscribe to file events
        let file_receiver = self.services.file_service.subscribe_to_events();
        *self.file_event_receiver.lock().await = Some(file_receiver);
        
        // Start file event forwarding task
        let event_tx = self.event_tx.clone();
        let file_receiver_clone = self.file_event_receiver.clone();
        let connection_id = self.connection_id.clone();
        
        tokio::spawn(async move {
            let mut receiver_guard = file_receiver_clone.lock().await;
            if let Some(ref mut receiver) = *receiver_guard {
                while let Ok(file_event) = receiver.recv().await {
                    if event_tx.send(WebSocketEvent::FileEvent(file_event)).is_err() {
                        debug!("File event receiver stopped for connection {}", connection_id);
                        break;
                    }
                }
            }
        });
        
        *file_subscribed = true;
        info!("Connection {} subscribed to file events", self.connection_id);
        Ok(())
    }
    
    async fn unsubscribe_from_files(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut file_subscribed = self.file_subscribed.write().await;
        
        if !*file_subscribed {
            debug!("Connection {} not subscribed to file events", self.connection_id);
            return Ok(());
        }
        
        // Clear file event receiver
        *self.file_event_receiver.lock().await = None;
        *file_subscribed = false;
        
        info!("Connection {} unsubscribed from file events", self.connection_id);
        Ok(())
    }
    
    async fn subscribe_to_compilation(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut compilation_subscribed = self.compilation_subscribed.write().await;
        
        if *compilation_subscribed {
            debug!("Connection {} already subscribed to compilation events", self.connection_id);
            return Ok(());
        }
        
        // Subscribe to enveloped compilation events
        let compilation_env_rx = self.services.latex_service.subscribe_to_compilation_envelopes();
        *self.compilation_env_receiver.lock().await = Some(compilation_env_rx);
        
        // Start compilation event forwarding task (enveloped)
        let event_tx = self.event_tx.clone();
        let compilation_env_receiver_clone = self.compilation_env_receiver.clone();
        let connection_id = self.connection_id.clone();
        
        tokio::spawn(async move {
            let mut receiver_guard = compilation_env_receiver_clone.lock().await;
            if let Some(ref mut receiver) = *receiver_guard {
                while let Ok(enveloped) = receiver.recv().await {
                    if event_tx.send(WebSocketEvent::CompilationEnvelope(enveloped)).is_err() {
                        debug!("Compilation event receiver stopped for connection {}", connection_id);
                        break;
                    }
                }
            }
        });
        
        *compilation_subscribed = true;
        info!("Connection {} subscribed to compilation events", self.connection_id);
        Ok(())
    }
    
    async fn unsubscribe_from_compilation(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut compilation_subscribed = self.compilation_subscribed.write().await;
        
        if !*compilation_subscribed {
            debug!("Connection {} not subscribed to compilation events", self.connection_id);
            return Ok(());
        }
        
        // Clear compilation event receiver
        *self.compilation_event_receiver.lock().await = None;
        *compilation_subscribed = false;
        
        info!("Connection {} unsubscribed from compilation events", self.connection_id);
        Ok(())
    }
    
    // Agent subscription methods removed - agent events now handled via SSE
    
    async fn handle_outgoing_events(
        &mut self,
        sender: &mut futures_util::stream::SplitSink<WebSocket, Message>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut event_rx = self.event_rx.lock().await.take()
            .ok_or("Event receiver already taken")?;
        
        while let Some(event) = event_rx.recv().await {
            let message = match event {
                WebSocketEvent::FileEvent(file_event) => {
                    let file_subscribed = *self.file_subscribed.read().await;
                    if !file_subscribed {
                        continue; // Skip if not subscribed
                    }
                    
                    serde_json::json!({
                        "type": "file_event",
                        "event": {
                            "event_type": match file_event.event_type {
                                crate::model::events::FileEventType::Created => "created",
                                crate::model::events::FileEventType::Modified => "modified",
                                crate::model::events::FileEventType::Deleted => "deleted",
                                crate::model::events::FileEventType::Renamed => "renamed",
                            },
                            "path": file_event.path,
                            "timestamp": file_event.timestamp,
                            "metadata": file_event.metadata
                        }
                    })
                }
                WebSocketEvent::CompilationEnvelope(enveloped) => {
                    let compilation_subscribed = *self.compilation_subscribed.read().await;
                    if !compilation_subscribed { continue; }
                    serde_json::json!({
                        "type": "event",
                        "topic": enveloped.topic,
                        "seq": enveloped.seq,
                        "ts": enveloped.ts,
                        "payload": {
                            "event_type": match enveloped.payload.event_type {
                                crate::model::events::CompilationEventType::Queued => "queued",
                                crate::model::events::CompilationEventType::Started => "started",
                                crate::model::events::CompilationEventType::Success => "success",
                                crate::model::events::CompilationEventType::Error => "error",
                                crate::model::events::CompilationEventType::MainFileDetected => "main_file_detected",
                            },
                            "main_file": enveloped.payload.main_file,
                            "timestamp": enveloped.payload.timestamp,
                            "metadata": enveloped.payload.metadata
                        }
                    })
                }
                // AgentEvent handling removed - now using SSE
                WebSocketEvent::ConnectionStatus { status, timestamp } => {
                    serde_json::json!({
                        "type": "connection",
                        "status": status,
                        "timestamp": timestamp
                    })
                }
            };
            
            let message_text = message.to_string();
            if let Err(e) = sender.send(Message::Text(message_text.into())).await {
                error!("Failed to send message to connection {}: {}", self.connection_id, e);
                return Err(e.into());
            }
        }
        
        info!("Event sender stopped for connection {}", self.connection_id);
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean up subscriptions
        *self.file_subscribed.write().await = false;
        *self.compilation_subscribed.write().await = false;
        *self.file_event_receiver.lock().await = None;
        *self.compilation_event_receiver.lock().await = None;
        
        info!("Cleaned up connection {}", self.connection_id);
        Ok(())
    }
}