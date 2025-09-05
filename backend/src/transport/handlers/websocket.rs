use crate::svc::FileService;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(service): State<Arc<FileService>>,
) -> Response {
    info!("WebSocket connection requested");
    ws.on_upgrade(move |socket| handle_websocket(socket, service))
}

async fn handle_websocket(socket: WebSocket, service: Arc<FileService>) {
    info!("WebSocket connection established");
    
    let (mut sender, mut receiver) = socket.split();
    let mut event_receiver = service.subscribe_to_events();
    
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
    let send_task = tokio::spawn(async move {
        while let Ok(file_event) = event_receiver.recv().await {
            debug!("Broadcasting file event: {:?}", file_event);
            
            let message = serde_json::json!({
                "type": "file_event",
                "event": file_event
            });
            
            if let Err(e) = sender.send(Message::Text(message.to_string())).await {
                error!("Failed to send file event: {}", e);
                break;
            }
        }
        debug!("Event sender task ended");
    });

    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received WebSocket message: {}", text);
                    
                    // Handle ping/pong or other client messages
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json) => {
                            if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
                                match msg_type {
                                    "ping" => {
                                        debug!("Received ping");
                                        // Pong will be sent automatically by the framework
                                    }
                                    "subscribe" => {
                                        debug!("Client subscribed to file events");
                                    }
                                    _ => {
                                        warn!("Unknown message type: {}", msg_type);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse WebSocket message as JSON: {}", e);
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received binary message, ignoring");
                }
                Ok(Message::Ping(payload)) => {
                    debug!("Received ping with payload: {:?}", payload);
                }
                Ok(Message::Pong(payload)) => {
                    debug!("Received pong with payload: {:?}", payload);
                }
                Ok(Message::Close(close_frame)) => {
                    info!("WebSocket connection closed: {:?}", close_frame);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
        debug!("Message receiver task ended");
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            debug!("Send task completed");
        }
        _ = receive_task => {
            debug!("Receive task completed");
        }
    }
    
    info!("WebSocket connection handler finished");
}