use wtransport::{Endpoint, Connection, ServerConfig, RecvStream, Identity, tls::{Certificate, CertificateChain, PrivateKey}};
use latex_ide_yrs_collab::{CollaborationEngine, UserInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, error, debug, warn};

// All types are defined in this file for simplicity

// All types are defined in this file

/// WebTransport message types for different streams
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebTransportMessage {
    // Stream 0: Document operations (Yrs sync)
    DocumentSync {
        document_id: Uuid,
        update: Vec<u8>,
        timestamp: i64,
    },
    
    // Stream 1: Real-time awareness (cursors, selections)
    Awareness {
        document_id: Uuid,
        user_id: Uuid,
        awareness_data: Vec<u8>,
    },
    
    // Stream 2: AI chat and communications  
    AiChat {
        conversation_id: Uuid,
        message: String,
        model_id: Option<String>,
    },
    
    // Stream 3: File operations
    FileOperation {
        document_id: Uuid,
        operation: FileOp,
    },
    
    // Stream 4: LaTeX compilation
    CompilationRequest {
        document_id: Uuid,
        engine: String,
        options: Vec<String>,
    },
    
    CompilationResult {
        document_id: Uuid,
        success: bool,
        pdf_data: Option<Vec<u8>>,
        log: String,
        errors: Vec<String>,
    },
    
    // Connection management
    Connect {
        user_info: UserInfo,
        document_id: Option<Uuid>,
    },
    
    Disconnect {
        user_id: Uuid,
        document_id: Option<Uuid>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

/// Stream types for WebTransport multiplexing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamType {
    DocumentSync = 0,
    Awareness = 1, 
    AiChat = 2,
    FileOps = 3,
    Compilation = 4,
}

impl StreamType {
    pub fn from_id(id: u64) -> Option<Self> {
        match id {
            0 => Some(StreamType::DocumentSync),
            1 => Some(StreamType::Awareness),
            2 => Some(StreamType::AiChat), 
            3 => Some(StreamType::FileOps),
            4 => Some(StreamType::Compilation),
            _ => None,
        }
    }
}

/// WebTransport server for LaTeX IDE collaboration
pub struct WebTransportServer {
    server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
    sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
    documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
    event_sender: broadcast::Sender<ServerEvent>,
}

#[derive(Clone, Debug)]
pub enum ServerEvent {
    UserConnected { user_id: Uuid, document_id: Option<Uuid> },
    UserDisconnected { user_id: Uuid, document_id: Option<Uuid> },
    DocumentUpdate { document_id: Uuid, update: Vec<u8> },
    AwarenessUpdate { document_id: Uuid, user_id: Uuid, awareness: Vec<u8> },
}

impl WebTransportServer {
    pub async fn new(bind_addr: &str, cert_path: &str, key_path: &str) -> Result<(Self, broadcast::Receiver<ServerEvent>)> {
        info!("Starting WebTransport server on {}", bind_addr);
        
        // Load TLS certificates
        let config = Self::create_server_config(cert_path, key_path).await?;
        
        // Create WebTransport endpoint
        let server = Endpoint::server(config)?;
        
        let (event_sender, event_receiver) = broadcast::channel(1000);
        
        let wt_server = Self {
            server,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        };
        
        Ok((wt_server, event_receiver))
    }
    
    pub async fn run(&self) -> Result<()> {
        info!("WebTransport server listening for connections");
        
        loop {
            let incoming_session = self.server.accept().await;
            let session_request = match incoming_session.await {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to accept session: {}", e);
                    continue;
                }
            };
            
            info!("New WebTransport connection");
            
            // Spawn task to handle this session
            let sessions = Arc::clone(&self.sessions);
            let documents = Arc::clone(&self.documents);
            let event_sender = self.event_sender.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_session(session_request, sessions, documents, event_sender).await {
                    error!("Session error: {}", e);
                }
            });
        }
    }
    
    async fn handle_session(
        session_request: wtransport::endpoint::SessionRequest,
        sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
        documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
        event_sender: broadcast::Sender<ServerEvent>,
    ) -> Result<()> {
        let session = session_request.accept().await?;
        debug!("WebTransport session established");
        
        // Wait for initial connect message
        let mut stream = session.accept_uni().await?;
        let connect_msg = Self::read_message(&mut stream).await?;
        
        let (user_info, document_id) = match connect_msg {
            WebTransportMessage::Connect { user_info, document_id } => (user_info, document_id),
            _ => {
                warn!("Expected Connect message, got: {:?}", connect_msg);
                return Err(anyhow::anyhow!("Invalid initial message"));
            }
        };
        
        info!("User {} connected to document {:?}", user_info.name, document_id);
        
        // Store user info
        let user_id = user_info.id;
        
        {
            let mut sessions_guard = sessions.write().await;
            sessions_guard.insert(user_id, user_info.clone());
        }
        
        // Get or create collaboration engine for document
        let collab_engine = if let Some(doc_id) = document_id {
            let mut documents_guard = documents.write().await;
            documents_guard.entry(doc_id)
                .or_insert_with(|| {
                    let (engine, _) = CollaborationEngine::new(user_info.clone());
                    Arc::new(Mutex::new(engine))
                })
                .clone()
        } else {
            let (engine, _) = CollaborationEngine::new(user_info.clone());
            Arc::new(Mutex::new(engine))
        };
        
        // Notify about user connection
        let _ = event_sender.send(ServerEvent::UserConnected { user_id, document_id });
        
        // Handle multiplexed streams
        Self::handle_multiplexed_streams(
            session.clone(),
            sessions.clone(),
            documents.clone(),
            event_sender.clone(),
            user_id,
            collab_engine,
        ).await?;
        
        // Cleanup on disconnect
        {
            let mut sessions_guard = sessions.write().await;
            sessions_guard.remove(&user_id);
        }
        
        let _ = event_sender.send(ServerEvent::UserDisconnected { user_id, document_id });
        info!("User {} disconnected", user_id);
        
        Ok(())
    }
    
    async fn handle_multiplexed_streams(
        session: Connection,
        _sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
        _documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
        event_sender: broadcast::Sender<ServerEvent>,
        _user_id: Uuid,
        collab_engine: Arc<Mutex<CollaborationEngine>>,
    ) -> Result<()> {
        loop {
            let mut stream = session.accept_uni().await?;
            // Simple stream identification (we'll just handle all as document sync for now)
            let stream_id = 0;
            
            debug!("Received stream {}", stream_id);
            
            let stream_type = StreamType::from_id(stream_id)
                .unwrap_or(StreamType::DocumentSync);
            
            // Handle different stream types
            match stream_type {
                StreamType::DocumentSync => {
                    Self::handle_document_sync_stream(
                        &mut stream,
                        collab_engine.clone(),
                        event_sender.clone(),
                    ).await?;
                }
                
                StreamType::Awareness => {
                    Self::handle_awareness_stream(
                        &mut stream,
                        collab_engine.clone(),
                        event_sender.clone(),
                    ).await?;
                }
                
                StreamType::AiChat => {
                    Self::handle_ai_chat_stream(&mut stream).await?;
                }
                
                StreamType::FileOps => {
                    Self::handle_file_ops_stream(&mut stream).await?;
                }
                
                StreamType::Compilation => {
                    Self::handle_compilation_stream(&mut stream).await?;
                }
            }
        }
    }
    
    async fn handle_document_sync_stream(
        stream: &mut RecvStream,
        collab_engine: Arc<Mutex<CollaborationEngine>>,
        event_sender: broadcast::Sender<ServerEvent>,
    ) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::DocumentSync { document_id, update, .. } => {
                debug!("Applying document update for {}", document_id);
                
                let engine = collab_engine.lock().await;
                engine.apply_update(&document_id, &update)?;
                
                let _ = event_sender.send(ServerEvent::DocumentUpdate { document_id, update });
            }
            _ => {
                warn!("Unexpected message on document sync stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn handle_awareness_stream(
        stream: &mut RecvStream,
        _collab_engine: Arc<Mutex<CollaborationEngine>>,
        event_sender: broadcast::Sender<ServerEvent>,
    ) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::Awareness { document_id, user_id, awareness_data } => {
                debug!("Updating awareness for user {} in document {}", user_id, document_id);
                
                let _ = event_sender.send(ServerEvent::AwarenessUpdate { 
                    document_id, 
                    user_id, 
                    awareness: awareness_data 
                });
            }
            _ => {
                warn!("Unexpected message on awareness stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn handle_ai_chat_stream(stream: &mut RecvStream) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::AiChat { conversation_id, message, model_id: _ } => {
                debug!("AI chat message for conversation {}: {}", conversation_id, message);
                // Forward to AI handler
                // TODO: Implement AI chat handling
            }
            _ => {
                warn!("Unexpected message on AI chat stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn handle_file_ops_stream(stream: &mut RecvStream) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::FileOperation { document_id, operation } => {
                debug!("File operation for document {}: {:?}", document_id, operation);
                // Handle file operations
                // TODO: Implement file handling
            }
            _ => {
                warn!("Unexpected message on file ops stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn handle_compilation_stream(stream: &mut RecvStream) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::CompilationRequest { document_id, engine, options } => {
                debug!("Compilation request for document {}: {} with options {:?}", 
                       document_id, engine, options);
                // Forward to LaTeX compiler
                // TODO: Implement compilation handling
            }
            _ => {
                warn!("Unexpected message on compilation stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn read_message(stream: &mut RecvStream) -> Result<WebTransportMessage> {
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes) as usize;
        
        let mut message_bytes = vec![0u8; length];
        stream.read_exact(&mut message_bytes).await?;
        
        let message: WebTransportMessage = bincode::deserialize(&message_bytes)?;
        Ok(message)
    }
    
    async fn create_server_config(cert_path: &str, key_path: &str) -> Result<ServerConfig> {
        // Load certificate and private key
        let cert_chain = Self::load_cert_chain(cert_path).await?;
        let private_key = Self::load_private_key(key_path).await?;
        
        // Create certificates from raw bytes
        let certs: Result<Vec<Certificate>, _> = cert_chain.into_iter()
            .map(|der| Certificate::from_der(der))
            .collect();
        let certs = certs?;
        
        // Create certificate chain and private key
        let cert_chain = CertificateChain::new(certs);
        let private_key = PrivateKey::from_der_pkcs8(private_key);
        let identity = Identity::new(cert_chain, private_key);
        
        let config = ServerConfig::builder()
            .with_bind_address(([0, 0, 0, 0], 3001).into())
            .with_identity(identity)
            .build();
        
        Ok(config)
    }
    
    async fn load_cert_chain(path: &str) -> Result<Vec<Vec<u8>>> {
        use rustls_pemfile;
        
        let cert_file = tokio::fs::read(path).await?;
        let mut reader = std::io::Cursor::new(cert_file);
        
        let certs = rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|cert| cert.to_vec())
            .collect();
            
        Ok(certs)
    }
    
    async fn load_private_key(path: &str) -> Result<Vec<u8>> {
        use rustls_pemfile;
        
        let key_file = tokio::fs::read(path).await?;
        let mut reader = std::io::Cursor::new(key_file);
        
        let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
            .collect::<Result<Vec<_>, _>>()?;
        
        match keys.into_iter().next() {
            Some(key) => Ok(key.secret_pkcs8_der().to_vec()),
            None => Err(anyhow::anyhow!("No private key found in file")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_type_conversion() {
        assert_eq!(StreamType::from_id(0), Some(StreamType::DocumentSync));
        assert_eq!(StreamType::from_id(1), Some(StreamType::Awareness));
        assert_eq!(StreamType::from_id(5), None);
    }
}