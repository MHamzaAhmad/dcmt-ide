use anyhow::Result;
use std::path::PathBuf;
use crate::types::*;

#[cfg(feature = "native-git")]
use latex_ide_git_manager::{
    GitRepository, SessionManager, GitOperations, HistoryViewer, ConflictResolver,
    GitStatus as NativeGitStatus, CommitInfo as NativeCommitInfo, 
    RollbackResult as NativeRollbackResult, ConflictInfo as NativeConflictInfo,
    ConflictType as NativeConflictType,
};

/// Desktop Git transport that uses direct git-manager operations for optimal performance
pub struct DesktopGitTransport {
    #[cfg(feature = "native-git")]
    git_repo: Option<GitRepository>,
    #[cfg(feature = "native-git")]
    session_manager: Option<SessionManager>,
    #[cfg(feature = "native-git")]
    git_operations: Option<GitOperations>,
    #[cfg(feature = "native-git")]
    history_viewer: Option<HistoryViewer>,
    #[cfg(feature = "native-git")]
    conflict_resolver: Option<ConflictResolver>,
    
    #[cfg(not(feature = "native-git"))]
    _placeholder: (),
}

impl DesktopGitTransport {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "native-git")]
            git_repo: None,
            #[cfg(feature = "native-git")]
            session_manager: None,
            #[cfg(feature = "native-git")]
            git_operations: None,
            #[cfg(feature = "native-git")]
            history_viewer: None,
            #[cfg(feature = "native-git")]
            conflict_resolver: None,
            
            #[cfg(not(feature = "native-git"))]
            _placeholder: (),
        }
    }
    
    /// Initialize with workspace path
    pub fn with_workspace(mut self, workspace_path: PathBuf) -> Result<Self> {
        #[cfg(feature = "native-git")]
        {
            let git_repo = GitRepository::new(workspace_path)?;
            let session_manager = SessionManager::new(git_repo.clone());
            let git_operations = GitOperations::new(git_repo.clone());
            let history_viewer = HistoryViewer::new(git_repo.clone());
            let conflict_resolver = ConflictResolver::new(git_repo.clone());
            
            self.git_repo = Some(git_repo);
            self.session_manager = Some(session_manager);
            self.git_operations = Some(git_operations);
            self.history_viewer = Some(history_viewer);
            self.conflict_resolver = Some(conflict_resolver);
        }
        
        Ok(self)
    }
    
    /// Initialize Git repository
    pub async fn init_repository(&mut self, path: String) -> Result<GitStatusResponse> {
        #[cfg(feature = "native-git")]
        {
            let workspace_path = PathBuf::from(path);
            *self = Self::new().with_workspace(workspace_path)?;
            self.get_status().await
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = path;
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Get current Git status
    pub async fn get_status(&self) -> Result<GitStatusResponse> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref git_repo) = self.git_repo {
                let status = git_repo.get_status()?;
                Ok(convert_git_status(status))
            } else {
                Err(anyhow::anyhow!("Git repository not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Start a Git session
    pub async fn start_session(&mut self) -> Result<String> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref mut session_manager) = self.session_manager {
                let session_branch = session_manager.start_session()?;
                Ok(session_branch)
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// End a Git session
    pub async fn end_session(&mut self, save_changes: bool) -> Result<()> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref mut session_manager) = self.session_manager {
                session_manager.end_session(save_changes)?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = save_changes;
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Stage all changes
    pub async fn stage_all_changes(&self) -> Result<()> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref git_operations) = self.git_operations {
                git_operations.stage_all_changes()?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Git operations not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Commit changes
    pub async fn commit(&self, message: String) -> Result<GitCommitInfo> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref session_manager) = self.session_manager {
                let commit_id = session_manager.commit_session_changes(&message)?;
                Ok(GitCommitInfo {
                    id: commit_id.to_string(),
                    short_id: format!("{:.7}", commit_id.to_string()),
                    message,
                    author_name: "Desktop User".to_string(),
                    author_email: "desktop@localhost".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    parents: vec![],
                    is_merge: false,
                    files_changed: vec![],
                    insertions: 0,
                    deletions: 0,
                })
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = message;
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Get commit history
    pub async fn get_commit_history(&self, branch_name: Option<String>, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref history_viewer) = self.history_viewer {
                let commits = history_viewer.get_commit_history(
                    branch_name.as_deref(), 
                    limit.unwrap_or(50)
                )?;
                let commit_infos: Vec<GitCommitInfo> = commits.into_iter()
                    .map(convert_commit_info)
                    .collect();
                Ok(commit_infos)
            } else {
                Err(anyhow::anyhow!("History viewer not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = (branch_name, limit);
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Get file history
    pub async fn get_file_history(&self, file_path: String, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref history_viewer) = self.history_viewer {
                let commits = history_viewer.get_file_history(&file_path, limit.unwrap_or(20))?;
                let commit_infos: Vec<GitCommitInfo> = commits.into_iter()
                    .map(convert_commit_info)
                    .collect();
                Ok(commit_infos)
            } else {
                Err(anyhow::anyhow!("History viewer not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = (file_path, limit);
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Get conflicts
    pub async fn get_conflicts(&self) -> Result<Vec<GitConflictInfo>> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref conflict_resolver) = self.conflict_resolver {
                let conflicts = conflict_resolver.get_current_conflicts()?;
                let conflict_infos: Vec<GitConflictInfo> = conflicts.into_iter()
                    .map(convert_conflict_info)
                    .collect();
                Ok(conflict_infos)
            } else {
                Err(anyhow::anyhow!("Conflict resolver not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Safe rollback to commit
    pub async fn safe_rollback_to_commit(&self, commit_id: String) -> Result<GitRollbackResult> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref git_operations) = self.git_operations {
                let result = git_operations.safe_rollback_to_commit(&commit_id)?;
                Ok(convert_rollback_result(result))
            } else {
                Err(anyhow::anyhow!("Git operations not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = commit_id;
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Get all available versions
    pub async fn get_all_versions(&self) -> Result<Vec<String>> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref session_manager) = self.session_manager {
                let versions = session_manager.get_all_versions()?;
                Ok(versions)
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Rollback to version
    pub async fn rollback_to_version(&mut self, version: String) -> Result<()> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref mut session_manager) = self.session_manager {
                session_manager.rollback_to_version(&version)?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = version;
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    /// Commit PDF version
    pub async fn commit_pdf_version(&self, pdf_path: String, latex_content: String) -> Result<GitCommitInfo> {
        #[cfg(feature = "native-git")]
        {
            if let Some(ref session_manager) = self.session_manager {
                let (commit_id, version) = session_manager.commit_pdf_version(&pdf_path, &latex_content)?;
                Ok(GitCommitInfo {
                    id: commit_id.to_string(),
                    short_id: format!("{:.7}", commit_id.to_string()),
                    message: format!("PDF compilation successful - {}", version),
                    author_name: "Desktop User".to_string(),
                    author_email: "desktop@localhost".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    parents: vec![],
                    is_merge: false,
                    files_changed: vec![pdf_path],
                    insertions: 0,
                    deletions: 0,
                })
            } else {
                Err(anyhow::anyhow!("Session manager not initialized"))
            }
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = (pdf_path, latex_content);
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
}

impl Default for DesktopGitTransport {
    fn default() -> Self {
        Self::new()
    }
}

// Conversion functions
#[cfg(feature = "native-git")]
fn convert_git_status(status: NativeGitStatus) -> GitStatusResponse {
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

#[cfg(feature = "native-git")]
fn convert_commit_info(commit: NativeCommitInfo) -> GitCommitInfo {
    GitCommitInfo {
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
    }
}

#[cfg(feature = "native-git")]
fn convert_rollback_result(result: NativeRollbackResult) -> GitRollbackResult {
    match result {
        NativeRollbackResult::Success { commit_id, commit_message } => {
            GitRollbackResult::Success { commit_id, commit_message }
        }
        NativeRollbackResult::HasUncommittedChanges(status) => {
            GitRollbackResult::HasUncommittedChanges(convert_git_status(status))
        }
        NativeRollbackResult::ConflictsDetected(_conflicts) => {
            // For now, treat conflicts as uncommitted changes
            // This would need more detailed implementation
            GitRollbackResult::HasUncommittedChanges(GitStatusResponse {
                current_branch: "unknown".to_string(),
                session_branch: None,
                has_changes: true,
                staged_files: vec![],
                modified_files: vec![],
                untracked_files: vec![],
                commits_ahead: 0,
                commits_behind: 0,
            })
        }
    }
}

#[cfg(feature = "native-git")]
fn convert_conflict_info(conflict: NativeConflictInfo) -> GitConflictInfo {
    GitConflictInfo {
        file_path: conflict.file_path,
        conflict_type: convert_conflict_type(conflict.conflict_type),
        our_content: conflict.our_content,
        their_content: conflict.their_content,
        base_content: conflict.base_content,
        merged_content: conflict.merged_content,
    }
}

#[cfg(feature = "native-git")]
fn convert_conflict_type(conflict_type: NativeConflictType) -> GitConflictType {
    match conflict_type {
        NativeConflictType::Content => GitConflictType::Content,
        NativeConflictType::ModifyDelete => GitConflictType::ModifyDelete,
        NativeConflictType::DeleteModify => GitConflictType::DeleteModify,
        NativeConflictType::AddAdd => GitConflictType::AddAdd,
    }
}