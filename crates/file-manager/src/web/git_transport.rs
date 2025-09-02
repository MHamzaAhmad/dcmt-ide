use anyhow::Result;

// Re-export transport types to maintain compatibility  
pub use super::transport::{GitOp, GitResponseData, GitStatusResponse, GitCommitInfo, 
                          GitConflictInfo, GitRollbackResult, GitConflictType};

/// Web-specific Git operations client that communicates via WebTransport
pub struct WebGitClient {
    // In a real implementation, this would hold WebTransport connection
    // For now we'll simulate the operations
}

impl WebGitClient {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Initialize Git repository on server
    pub async fn init_repository(&self, path: String) -> Result<GitStatusResponse> {
        let operation = GitOp::InitRepository { path };
        match self.send_git_operation(operation).await? {
            GitResponseData::Status(status) => Ok(status),
            _ => Err(anyhow::anyhow!("Unexpected response for init repository")),
        }
    }
    
    /// Get current Git status
    pub async fn get_status(&self) -> Result<GitStatusResponse> {
        let operation = GitOp::GetStatus;
        match self.send_git_operation(operation).await? {
            GitResponseData::Status(status) => Ok(status),
            _ => Err(anyhow::anyhow!("Unexpected response for get status")),
        }
    }
    
    /// Start a Git session
    pub async fn start_session(&self) -> Result<String> {
        let operation = GitOp::StartSession;
        match self.send_git_operation(operation).await? {
            GitResponseData::SessionBranch(branch) => Ok(branch),
            _ => Err(anyhow::anyhow!("Unexpected response for start session")),
        }
    }
    
    /// End a Git session
    pub async fn end_session(&self, save_changes: bool) -> Result<()> {
        let operation = GitOp::EndSession { save_changes };
        let _response = self.send_git_operation(operation).await?;
        Ok(())
    }
    
    /// Stage all changes
    pub async fn stage_all_changes(&self) -> Result<()> {
        let operation = GitOp::StageAllChanges;
        let _response = self.send_git_operation(operation).await?;
        Ok(())
    }
    
    /// Commit changes
    pub async fn commit(&self, message: String) -> Result<GitCommitInfo> {
        let operation = GitOp::Commit { message };
        match self.send_git_operation(operation).await? {
            GitResponseData::CommitDetails(commit) => Ok(commit),
            _ => Err(anyhow::anyhow!("Unexpected response for commit")),
        }
    }
    
    /// Get commit history
    pub async fn get_commit_history(&self, branch_name: Option<String>, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        let operation = GitOp::GetCommitHistory { branch_name, limit };
        match self.send_git_operation(operation).await? {
            GitResponseData::CommitHistory(commits) => Ok(commits),
            _ => Err(anyhow::anyhow!("Unexpected response for commit history")),
        }
    }
    
    /// Get file history
    pub async fn get_file_history(&self, file_path: String, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        let operation = GitOp::GetFileHistory { file_path, limit };
        match self.send_git_operation(operation).await? {
            GitResponseData::FileHistory(commits) => Ok(commits),
            _ => Err(anyhow::anyhow!("Unexpected response for file history")),
        }
    }
    
    /// Get conflicts
    pub async fn get_conflicts(&self) -> Result<Vec<GitConflictInfo>> {
        let operation = GitOp::GetConflicts;
        match self.send_git_operation(operation).await? {
            GitResponseData::Conflicts(conflicts) => Ok(conflicts),
            _ => Err(anyhow::anyhow!("Unexpected response for get conflicts")),
        }
    }
    
    /// Safe rollback to commit
    pub async fn safe_rollback_to_commit(&self, commit_id: String) -> Result<GitRollbackResult> {
        let operation = GitOp::SafeRollbackToCommit { commit_id };
        match self.send_git_operation(operation).await? {
            GitResponseData::RollbackResult(result) => Ok(result),
            _ => Err(anyhow::anyhow!("Unexpected response for rollback")),
        }
    }
    
    /// Send Git operation to server via WebTransport
    async fn send_git_operation(&self, operation: GitOp) -> Result<GitResponseData> {
        use super::transport;
        
        #[cfg(target_arch = "wasm32")]
        {
            tracing::debug!("Sending Git operation: {:?}", operation);
            
            transport::send_git_operation(operation)
                .await
                .map_err(|e| anyhow::anyhow!("Transport error: {}", e))
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            tracing::info!("Native git operation: {:?}", operation);
            // For native builds, we could either:
            // 1. Also use transport layer for consistency 
            // 2. Use direct git operations for better performance
            // For now, let's use transport layer for consistency
            Err(anyhow::anyhow!("Native git operations via transport not implemented yet"))
        }
    }
}

impl Default for WebGitClient {
    fn default() -> Self {
        Self::new()
    }
}