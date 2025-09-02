use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, error, debug, warn};
use wtransport::{Endpoint, Connection};
use latex_ide_yrs_collab::{CollaborationEngine, UserInfo};

use crate::types::{WebTransportMessage, ServerEvent, StreamType};
use crate::utils::{create_server_config, read_message};
use crate::git_manager::GitServerManager;
use crate::handlers::{
    handle_document_sync_stream, handle_awareness_stream, handle_ai_chat_stream,
    handle_file_ops_stream, handle_compilation_stream, handle_git_ops_stream
};

/// WebTransport server for LaTeX IDE collaboration
pub struct WebTransportServer {
    server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
    sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
    documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
    git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>, // keyed by workspace path
    event_sender: broadcast::Sender<ServerEvent>,
}

impl WebTransportServer {
    pub async fn new(bind_addr: &str, cert_path: &str, key_path: &str) -> Result<(Self, broadcast::Receiver<ServerEvent>)> {
        info!("Starting WebTransport server on {}", bind_addr);
        
        // Load TLS certificates
        let config = create_server_config(cert_path, key_path).await?;
        
        // Create WebTransport endpoint
        let server = Endpoint::server(config)?;
        
        let (event_sender, event_receiver) = broadcast::channel(1000);
        
        let wt_server = Self {
            server,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            git_managers: Arc::new(RwLock::new(HashMap::new())),
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
            let git_managers = Arc::clone(&self.git_managers);
            let event_sender = self.event_sender.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_session(session_request, sessions, documents, git_managers, event_sender).await {
                    error!("Session error: {}", e);
                }
            });
        }
    }
    
    async fn handle_session(
        session_request: wtransport::endpoint::SessionRequest,
        sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
        documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
        git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
        event_sender: broadcast::Sender<ServerEvent>,
    ) -> Result<()> {
        let session = session_request.accept().await?;
        debug!("WebTransport session established");
        
        // Wait for initial connect message
        let mut stream = session.accept_uni().await?;
        let connect_msg = read_message(&mut stream).await?;
        
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
            git_managers.clone(),
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
        git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
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
                    handle_document_sync_stream(
                        &mut stream,
                        collab_engine.clone(),
                        event_sender.clone(),
                    ).await?;
                }
                
                StreamType::Awareness => {
                    handle_awareness_stream(
                        &mut stream,
                        collab_engine.clone(),
                        event_sender.clone(),
                    ).await?;
                }
                
                StreamType::AiChat => {
                    handle_ai_chat_stream(&mut stream).await?;
                }
                
                StreamType::FileOps => {
                    handle_file_ops_stream(&mut stream).await?;
                }
                
                StreamType::Compilation => {
                    handle_compilation_stream(&mut stream).await?;
                }
                
                StreamType::GitOps => {
                    handle_git_ops_stream(&mut stream, git_managers.clone()).await?;
                }
            }
        }
    }
}