use serde::{Deserialize, Serialize};
use uuid::Uuid;
use latex_ide_yrs_collab::UserInfo;

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
    
    // Version management
    GetAllVersions,
    RollbackToVersion { version: String },
    
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
    VersionList(Vec<String>),
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

#[derive(Clone, Debug)]
pub enum ServerEvent {
    UserConnected { user_id: Uuid, document_id: Option<Uuid> },
    UserDisconnected { user_id: Uuid, document_id: Option<Uuid> },
    DocumentUpdate { document_id: Uuid, update: Vec<u8> },
    AwarenessUpdate { document_id: Uuid, user_id: Uuid, awareness: Vec<u8> },
}