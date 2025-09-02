use wtransport::{Endpoint, Connection, ServerConfig, RecvStream, Identity, tls::{Certificate, CertificateChain, PrivateKey}};
use latex_ide_yrs_collab::{CollaborationEngine, UserInfo};
// Git integration temporarily disabled for Docker compatibility
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
    
    // Stream 5: Git version control operations
    GitOperation {
        operation: GitOp,
    },
    
    GitResponse {
        success: bool,
        data: Option<GitResponseData>,
        error: Option<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitOp {
    // Repository management
    InitRepository { path: String },
    GetStatus,
    GetBranches,
    GetCurrentBranch,
    
    // Session management
    StartSession,
    EndSession { save_changes: bool },
    
    // File operations
    StageFile { file_path: String },
    UnstageFile { file_path: String },
    StageAllChanges,
    DiscardFileChanges { file_path: String },
    DiscardAllChanges,
    
    // Branch operations
    CreateBranch { branch_name: String, from_current: bool },
    DeleteBranch { branch_name: String, force: bool },
    CheckoutBranch { branch_name: String },
    MergeBranch { branch_name: String, message: Option<String> },
    
    // Commit operations
    Commit { message: String },
    CommitPdfVersion { pdf_path: String, latex_content: String },
    GetCommitHistory { branch_name: Option<String>, limit: Option<usize> },
    GetCommitDetails { commit_id: String },
    
    // Conflict resolution
    GetConflicts,
    ResolveConflicts { resolutions: Vec<GitResolutionChoice> },
    AbortMerge,
    
    // History and rollback
    GetFileHistory { file_path: String, limit: Option<usize> },
    SearchCommits { query: String, limit: Option<usize> },
    SafeRollbackToCommit { commit_id: String },
    
    // Remote operations
    PushToRemote { remote_name: String, branch_name: String },
    PullFromRemote { remote_name: String, branch_name: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitResolutionChoice {
    pub file_path: String,
    pub resolution: GitResolution,
    pub custom_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitResolution {
    TakeOurs,
    TakeTheirs,
    TakeBase,
    Custom,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitResponseData {
    Status(GitStatusResponse),
    Branches(Vec<String>),
    CurrentBranch(String),
    SessionBranch(String),
    CommitHistory(Vec<GitCommitInfo>),
    CommitDetails(GitCommitInfo),
    Conflicts(Vec<GitConflictInfo>),
    FileHistory(Vec<GitCommitInfo>),
    SearchResults(Vec<GitCommitInfo>),
    RollbackResult(GitRollbackResult),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitStatusResponse {
    pub current_branch: String,
    pub session_branch: Option<String>,
    pub has_changes: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String, // ISO format string for web compatibility
    pub parents: Vec<String>,
    pub is_merge: bool,
    pub files_changed: Vec<String>,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitConflictInfo {
    pub file_path: String,
    pub conflict_type: GitConflictType,
    pub our_content: Option<String>,
    pub their_content: Option<String>,
    pub base_content: Option<String>,
    pub merged_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitConflictType {
    Content,
    ModifyDelete,
    DeleteModify,
    AddAdd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GitRollbackResult {
    Success {
        commit_id: String,
        commit_message: String,
    },
    HasUncommittedChanges(GitStatusResponse),
    ConflictsDetected(Vec<GitConflictInfo>),
}

/// Git server manager that wraps all Git operations for a workspace
pub struct GitServerManager {
    pub git_repo: GitRepository,
    pub session_manager: Option<SessionManager>,
    pub git_operations: GitOperations,
    pub history_viewer: HistoryViewer,
    pub conflict_resolver: ConflictResolver,
}

impl GitServerManager {
    pub fn new(workspace_path: std::path::PathBuf) -> Result<Self> {
        let git_repo = GitRepository::new(workspace_path)?;
        let session_manager = Some(SessionManager::new(git_repo.clone()));
        let git_operations = GitOperations::new(git_repo.clone());
        let history_viewer = HistoryViewer::new(git_repo.clone());
        let conflict_resolver = ConflictResolver::new(git_repo.clone());

        Ok(Self {
            git_repo,
            session_manager,
            git_operations,
            history_viewer,
            conflict_resolver,
        })
    }
}

/// Stream types for WebTransport multiplexing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamType {
    DocumentSync = 0,
    Awareness = 1, 
    AiChat = 2,
    FileOps = 3,
    Compilation = 4,
    GitOps = 5,
}

impl StreamType {
    pub fn from_id(id: u64) -> Option<Self> {
        match id {
            0 => Some(StreamType::DocumentSync),
            1 => Some(StreamType::Awareness),
            2 => Some(StreamType::AiChat), 
            3 => Some(StreamType::FileOps),
            4 => Some(StreamType::Compilation),
            5 => Some(StreamType::GitOps),
            _ => None,
        }
    }
}

/// WebTransport server for LaTeX IDE collaboration
pub struct WebTransportServer {
    server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
    sessions: Arc<RwLock<HashMap<Uuid, UserInfo>>>,
    documents: Arc<RwLock<HashMap<Uuid, Arc<Mutex<CollaborationEngine>>>>>,
    git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>, // keyed by workspace path
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
                
                StreamType::GitOps => {
                    Self::handle_git_ops_stream(&mut stream, git_managers.clone()).await?;
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
    
    async fn handle_git_ops_stream(
        stream: &mut RecvStream,
        git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
    ) -> Result<()> {
        let message = Self::read_message(stream).await?;
        
        match message {
            WebTransportMessage::GitOperation { operation } => {
                debug!("Git operation request: {:?}", operation);
                
                let response = Self::execute_git_operation(operation, git_managers).await;
                
                // Send response back (for now we'll just log it)
                match response {
                    Ok(data) => {
                        debug!("Git operation completed successfully: {:?}", data);
                        // TODO: Send GitResponse back to client
                    }
                    Err(e) => {
                        error!("Git operation failed: {}", e);
                        // TODO: Send error GitResponse back to client
                    }
                }
            }
            _ => {
                warn!("Unexpected message on git ops stream: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    async fn execute_git_operation(
        operation: GitOp,
        git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
    ) -> Result<Option<GitResponseData>> {
        use std::path::PathBuf;
        
        // For now, we'll use current directory as workspace. In production, 
        // this should be determined from the client session context.
        let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let workspace_key = workspace_path.to_string_lossy().to_string();
        
        // Get or create git manager for this workspace
        let git_manager = {
            let mut managers = git_managers.write().await;
            managers.entry(workspace_key.clone())
                .or_insert_with(|| {
                    match GitServerManager::new(workspace_path.clone()) {
                        Ok(manager) => Arc::new(Mutex::new(manager)),
                        Err(e) => {
                            error!("Failed to create git manager: {}", e);
                            // Return a dummy manager that will fail operations
                            Arc::new(Mutex::new(GitServerManager {
                                git_repo: GitRepository::new(workspace_path).unwrap_or_else(|_| 
                                    GitRepository::new(PathBuf::from(".")).unwrap()
                                ),
                                session_manager: None,
                                git_operations: GitOperations::new(GitRepository::new(PathBuf::from(".")).unwrap()),
                                history_viewer: HistoryViewer::new(GitRepository::new(PathBuf::from(".")).unwrap()),
                                conflict_resolver: ConflictResolver::new(GitRepository::new(PathBuf::from(".")).unwrap()),
                            }))
                        }
                    }
                })
                .clone()
        };
        
        let mut manager = git_manager.lock().await;
        
        match operation {
            GitOp::InitRepository { path: _ } => {
                // Repository should already be initialized in GitServerManager::new
                let status = manager.git_repo.get_status()?;
                Ok(Some(GitResponseData::Status(Self::convert_git_status(status))))
            }
            
            GitOp::GetStatus => {
                let status = manager.git_repo.get_status()?;
                Ok(Some(GitResponseData::Status(Self::convert_git_status(status))))
            }
            
            GitOp::GetBranches => {
                let branches = manager.git_operations.list_branches()?;
                Ok(Some(GitResponseData::Branches(branches)))
            }
            
            GitOp::GetCurrentBranch => {
                let current = manager.git_operations.get_current_branch()?;
                Ok(Some(GitResponseData::CurrentBranch(current)))
            }
            
            GitOp::StartSession => {
                if let Some(ref mut session_mgr) = manager.session_manager {
                    let session_branch = session_mgr.start_session()?;
                    Ok(Some(GitResponseData::SessionBranch(session_branch)))
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::EndSession { save_changes } => {
                if let Some(ref mut session_mgr) = manager.session_manager {
                    session_mgr.end_session(save_changes)?;
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::StageFile { file_path } => {
                manager.git_operations.stage_file(&file_path)?;
                Ok(None)
            }
            
            GitOp::UnstageFile { file_path } => {
                manager.git_operations.unstage_file(&file_path)?;
                Ok(None)
            }
            
            GitOp::StageAllChanges => {
                manager.git_operations.stage_all_changes()?;
                Ok(None)
            }
            
            GitOp::Commit { message } => {
                if let Some(ref session_mgr) = manager.session_manager {
                    let commit_id = session_mgr.commit_session_changes(&message)?;
                    Ok(Some(GitResponseData::CommitDetails(GitCommitInfo {
                        id: commit_id.to_string(),
                        short_id: format!("{:.7}", commit_id.to_string()),
                        message,
                        author_name: "Server".to_string(),
                        author_email: "server@localhost".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        parents: vec![],
                        is_merge: false,
                        files_changed: vec![],
                        insertions: 0,
                        deletions: 0,
                    })))
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::CommitPdfVersion { pdf_path, latex_content } => {
                if let Some(ref mut session_mgr) = manager.session_manager {
                    let (commit_id, version) = session_mgr.commit_pdf_version(&pdf_path, &latex_content)?;
                    Ok(Some(GitResponseData::CommitDetails(GitCommitInfo {
                        id: commit_id.to_string(),
                        short_id: format!("{:.7}", commit_id.to_string()),
                        message: format!("PDF version {}", version),
                        author_name: "Server".to_string(),
                        author_email: "server@localhost".to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        parents: vec![],
                        is_merge: false,
                        files_changed: vec![pdf_path],
                        insertions: 0,
                        deletions: 0,
                    })))
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::GetCommitHistory { branch_name, limit } => {
                let commits = manager.history_viewer.get_commit_history(
                    branch_name.as_deref(), 
                    limit.unwrap_or(50)
                )?;
                let commit_infos: Vec<GitCommitInfo> = commits.into_iter()
                    .map(|commit| GitCommitInfo {
                        id: commit.id,
                        short_id: commit.short_id,
                        message: commit.message,
                        author_name: commit.author_name,
                        author_email: commit.author_email,
                        timestamp: commit.timestamp.to_rfc3339(),
                        parents: commit.parents,
                        is_merge: commit.is_merge,
                        files_changed: commit.files_changed,
                        insertions: commit.insertions,
                        deletions: commit.deletions,
                    })
                    .collect();
                Ok(Some(GitResponseData::CommitHistory(commit_infos)))
            }
            
            GitOp::SafeRollbackToCommit { commit_id } => {
                let result = manager.git_operations.safe_rollback_to_commit(&commit_id)?;
                match result {
                    latex_ide_git_manager::RollbackResult::Success { commit_id, commit_message } => {
                        Ok(Some(GitResponseData::RollbackResult(GitRollbackResult::Success {
                            commit_id,
                            commit_message,
                        })))
                    }
                    latex_ide_git_manager::RollbackResult::HasUncommittedChanges(status) => {
                        Ok(Some(GitResponseData::RollbackResult(GitRollbackResult::HasUncommittedChanges(
                            Self::convert_git_status(status)
                        ))))
                    }
                    latex_ide_git_manager::RollbackResult::ConflictsDetected(conflicts) => {
                        let conflict_infos: Vec<GitConflictInfo> = conflicts.into_iter()
                            .map(|conflict| GitConflictInfo {
                                file_path: conflict.file_path,
                                conflict_type: match conflict.conflict_type {
                                    latex_ide_git_manager::ConflictType::Content => GitConflictType::Content,
                                    latex_ide_git_manager::ConflictType::ModifyDelete => GitConflictType::ModifyDelete,
                                    latex_ide_git_manager::ConflictType::DeleteModify => GitConflictType::DeleteModify,
                                    latex_ide_git_manager::ConflictType::AddAdd => GitConflictType::AddAdd,
                                },
                                our_content: conflict.our_content,
                                their_content: conflict.their_content,
                                base_content: conflict.base_content,
                                merged_content: conflict.merged_content,
                            })
                            .collect();
                        Ok(Some(GitResponseData::RollbackResult(GitRollbackResult::ConflictsDetected(conflict_infos))))
                    }
                }
            }
            
            GitOp::CreateBranch { branch_name, from_current } => {
                manager.git_operations.create_branch(&branch_name, from_current)?;
                Ok(None)
            }
            
            GitOp::CheckoutBranch { branch_name } => {
                manager.git_operations.checkout_branch(&branch_name)?;
                Ok(None)
            }
            
            GitOp::DeleteBranch { branch_name, force } => {
                manager.git_operations.delete_branch(&branch_name, force)?;
                Ok(None)
            }
            
            GitOp::DiscardFileChanges { file_path } => {
                manager.git_operations.discard_file_changes(&file_path)?;
                Ok(None)
            }
            
            GitOp::DiscardAllChanges => {
                manager.git_operations.discard_all_changes()?;
                Ok(None)
            }
            
            GitOp::GetConflicts => {
                let conflicts = manager.conflict_resolver.get_current_conflicts()?;
                let conflict_infos: Vec<GitConflictInfo> = conflicts.into_iter()
                    .map(|conflict| GitConflictInfo {
                        file_path: conflict.file_path,
                        conflict_type: match conflict.conflict_type {
                            latex_ide_git_manager::ConflictType::Content => GitConflictType::Content,
                            latex_ide_git_manager::ConflictType::ModifyDelete => GitConflictType::ModifyDelete,
                            latex_ide_git_manager::ConflictType::DeleteModify => GitConflictType::DeleteModify,
                            latex_ide_git_manager::ConflictType::AddAdd => GitConflictType::AddAdd,
                        },
                        our_content: conflict.our_content,
                        their_content: conflict.their_content,
                        base_content: conflict.base_content,
                        merged_content: conflict.merged_content,
                    })
                    .collect();
                Ok(Some(GitResponseData::Conflicts(conflict_infos)))
            }
            
            GitOp::ResolveConflicts { resolutions } => {
                let git_resolutions: Vec<latex_ide_git_manager::ResolutionChoice> = resolutions.into_iter()
                    .map(|res| latex_ide_git_manager::ResolutionChoice {
                        file_path: res.file_path,
                        resolution: match res.resolution {
                            GitResolution::TakeOurs => latex_ide_git_manager::Resolution::TakeOurs,
                            GitResolution::TakeTheirs => latex_ide_git_manager::Resolution::TakeTheirs,
                            GitResolution::TakeBase => latex_ide_git_manager::Resolution::TakeBase,
                            GitResolution::Custom => latex_ide_git_manager::Resolution::Custom,
                            GitResolution::Manual => latex_ide_git_manager::Resolution::Manual,
                        },
                        custom_content: res.custom_content,
                    })
                    .collect();
                manager.conflict_resolver.resolve_conflicts(git_resolutions)?;
                Ok(None)
            }
            
            GitOp::GetFileHistory { file_path, limit } => {
                let commits = manager.history_viewer.get_file_history(&file_path, limit.unwrap_or(20))?;
                let commit_infos: Vec<GitCommitInfo> = commits.into_iter()
                    .map(|commit| GitCommitInfo {
                        id: commit.id,
                        short_id: commit.short_id,
                        message: commit.message,
                        author_name: commit.author_name,
                        author_email: commit.author_email,
                        timestamp: commit.timestamp.to_rfc3339(),
                        parents: commit.parents,
                        is_merge: commit.is_merge,
                        files_changed: commit.files_changed,
                        insertions: commit.insertions,
                        deletions: commit.deletions,
                    })
                    .collect();
                Ok(Some(GitResponseData::FileHistory(commit_infos)))
            }
            
            // Remote operations (placeholder implementations)
            GitOp::PushToRemote { remote_name: _, branch_name: _ } => {
                warn!("Remote operations not yet implemented in server");
                Err(anyhow::anyhow!("Remote operations not yet implemented"))
            }
            
            GitOp::PullFromRemote { remote_name: _, branch_name: _ } => {
                warn!("Remote operations not yet implemented in server");
                Err(anyhow::anyhow!("Remote operations not yet implemented"))
            }
            
            // Operations that don't need implementation
            GitOp::SearchCommits { query: _, limit: _ } => {
                warn!("Search commits not yet implemented");
                Err(anyhow::anyhow!("Search commits not yet implemented"))
            }
            
            GitOp::GetCommitDetails { commit_id: _ } => {
                warn!("Get commit details not yet implemented");
                Err(anyhow::anyhow!("Get commit details not yet implemented"))
            }
            
            GitOp::MergeBranch { branch_name: _, message: _ } => {
                warn!("Merge branch not yet implemented");
                Err(anyhow::anyhow!("Merge branch not yet implemented"))
            }
            
            GitOp::AbortMerge => {
                warn!("Abort merge not yet implemented");
                Err(anyhow::anyhow!("Abort merge not yet implemented"))
            }
        }
    }
    
    fn convert_git_status(status: latex_ide_git_manager::GitStatus) -> GitStatusResponse {
        GitStatusResponse {
            current_branch: status.current_branch,
            session_branch: status.session_branch,
            has_changes: status.has_changes,
            staged_files: status.staged_files,
            modified_files: status.modified_files,
            untracked_files: status.untracked_files,
            commits_ahead: status.commits_ahead,
            commits_behind: status.commits_behind,
        }
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