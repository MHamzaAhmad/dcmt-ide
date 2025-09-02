use serde::{Serialize, Deserialize};

/// Git operations that can be performed via transport layer
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    
    // Commit operations
    Commit { message: String },
    CommitPdfVersion { pdf_path: String, latex_content: String },
    GetCommitHistory { branch_name: Option<String>, limit: Option<usize> },
    
    // History and rollback
    GetFileHistory { file_path: String, limit: Option<usize> },
    SafeRollbackToCommit { commit_id: String },
    
    // Version management
    GetAllVersions,
    RollbackToVersion { version: String },
    
    // Conflicts
    GetConflicts,
}

/// Response data from Git operations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GitResponseData {
    Status(GitStatusResponse),
    Branches(Vec<String>),
    CurrentBranch(String),
    SessionBranch(String),
    CommitHistory(Vec<GitCommitInfo>),
    CommitDetails(GitCommitInfo),
    FileHistory(Vec<GitCommitInfo>),
    RollbackResult(GitRollbackResult),
    Conflicts(Vec<GitConflictInfo>),
    VersionList(Vec<String>),
}

/// Git repository status information
#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// Git commit information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitCommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub parents: Vec<String>,
    pub is_merge: bool,
    pub files_changed: Vec<String>,
    pub insertions: u32,
    pub deletions: u32,
}

/// Rollback operation result
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GitRollbackResult {
    Success {
        commit_id: String,
        commit_message: String,
    },
    HasUncommittedChanges(GitStatusResponse),
}

/// Git merge conflict information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitConflictInfo {
    pub file_path: String,
    pub conflict_type: GitConflictType,
    pub our_content: Option<String>,
    pub their_content: Option<String>,
    pub base_content: Option<String>,
    pub merged_content: Option<String>,
}

/// Types of Git conflicts
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GitConflictType {
    Content,
    ModifyDelete,
    DeleteModify,
    AddAdd,
}

/// Transport message for communication with backend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitTransportMessage {
    pub operation: GitOp,
}

/// Transport response message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitTransportResponse {
    pub success: bool,
    pub data: Option<GitResponseData>,
    pub error: Option<String>,
}