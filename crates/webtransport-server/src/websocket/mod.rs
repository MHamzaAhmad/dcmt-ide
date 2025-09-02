pub mod file_handler;
pub mod compilation_handler;
pub mod git_handler;

use axum::extract::ws::{WebSocket, Message};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn};

use crate::types::{FileOp, GitOp, GitResponseData};
use file_handler::handle_file_operation;
use compilation_handler::handle_compilation_request;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransportMessage {
    FileOperation { operation: FileOp },
    CompilationRequest { document_id: String, content: String, engine: String },
    CompilationResult { document_id: String, success: bool, pdf_data: Option<Vec<u8>>, log: String },
    GitOperation { operation: GitOp },
    GitResponse { success: bool, data: Option<GitResponseData>, error: Option<String> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

pub async fn handle_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("WebSocket connection established");
    
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Binary(data)) = msg {
            match serde_json::from_slice::<TransportMessage>(&data) {
                Ok(transport_msg) => {
                    info!("Received transport message: {:?}", transport_msg);
                    
                    let response = match transport_msg {
                        TransportMessage::FileOperation { operation } => {
                            handle_file_operation(operation).await
                        }
                        TransportMessage::CompilationRequest { document_id, content, engine } => {
                            handle_compilation_request(document_id, content, engine).await
                        }
                        #[cfg(feature = "native-git")]
                        TransportMessage::GitOperation { operation } => {
                            git_handler::handle_git_operation(operation).await
                        }
                        #[cfg(not(feature = "native-git"))]
                        TransportMessage::GitOperation { .. } => {
                            TransportMessage::GitResponse {
                                success: false,
                                data: None,
                                error: Some("Git operations not available in this build".to_string()),
                            }
                        }
                        _ => {
                            warn!("Unsupported message type");
                            continue;
                        }
                    };
                    
                    if let Ok(response_data) = serde_json::to_vec(&response) {
                        if let Err(e) = sender.send(Message::Binary(response_data)).await {
                            error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse transport message: {}", e);
                }
            }
        } else if let Ok(Message::Close(_)) = msg {
            info!("WebSocket connection closed");
            break;
        }
    }
}