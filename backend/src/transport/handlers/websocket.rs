use crate::transport::routes::websocket::WebSocketServices;
use crate::model::events::FileEvent;
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
    
    // Send initial connection confirmation
    if let Err(e) = sender
        .send(Message::Text(
            serde_json::json!({
                "type": "connection",
                "status": "connected",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            })
            .to_string(),
        ))
        .await
    {
        error!("Failed to send connection confirmation: {}", e);
        return;
    }

    // Handle incoming messages and outgoing events concurrently
    let send_task = {
        let mut state = connection_state.clone();
        let conn_id = connection_id.clone();
        tokio::spawn(async move {
            if let Err(e) = state.handle_outgoing_events(&mut sender).await {
                error!("Event sender task failed for connection {}: {}", conn_id, e);
            }
            debug!("Event sender task ended for connection {}", conn_id);
        })
    };

    let receive_task = {
        let mut state = connection_state.clone();
        let conn_id = connection_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = state.handle_incoming_message(text).await {
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
    file_event_receiver: Arc<tokio::sync::Mutex<Option<broadcast::Receiver<FileEvent>>>>,
    event_tx: Arc<tokio::sync::mpsc::UnboundedSender<WebSocketEvent>>,
    event_rx: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<WebSocketEvent>>>>,
}

#[derive(Debug, Clone)]
enum WebSocketEvent {
    FileEvent(FileEvent),
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
            file_event_receiver: Arc::new(tokio::sync::Mutex::new(None)),
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
            if let Err(e) = sender.send(Message::Text(message_text)).await {
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
        *self.file_event_receiver.lock().await = None;
        
        info!("Cleaned up connection {}", self.connection_id);
        Ok(())
    }
}