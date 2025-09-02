pub mod types;

// Platform-specific modules
#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;

// Re-export types for easy access
pub use types::*;

// Re-export platform-specific clients
#[cfg(target_arch = "wasm32")]
pub use web::WebGitTransport as GitTransport;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::DesktopGitTransport as GitTransport;

// Unified interface that works on both platforms
use anyhow::Result;

/// Unified Git transport interface that works on both web and desktop
pub struct UnifiedGitTransport {
    #[cfg(target_arch = "wasm32")]
    inner: web::WebGitTransport,
    
    #[cfg(not(target_arch = "wasm32"))]
    inner: desktop::DesktopGitTransport,
}

impl UnifiedGitTransport {
    pub fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            inner: web::WebGitTransport::new(),
            
            #[cfg(not(target_arch = "wasm32"))]
            inner: desktop::DesktopGitTransport::new(),
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_workspace(workspace_path: std::path::PathBuf) -> Result<Self> {
        Ok(Self {
            inner: desktop::DesktopGitTransport::new().with_workspace(workspace_path)?,
        })
    }
    
    /// Initialize Git repository
    pub async fn init_repository(&mut self, path: String) -> Result<GitStatusResponse> {
        self.inner.init_repository(path).await
    }
    
    /// Get current Git status
    pub async fn get_status(&self) -> Result<GitStatusResponse> {
        self.inner.get_status().await
    }
    
    /// Start a Git session
    pub async fn start_session(&mut self) -> Result<String> {
        self.inner.start_session().await
    }
    
    /// End a Git session
    pub async fn end_session(&mut self, save_changes: bool) -> Result<()> {
        self.inner.end_session(save_changes).await
    }
    
    /// Stage all changes
    pub async fn stage_all_changes(&self) -> Result<()> {
        self.inner.stage_all_changes().await
    }
    
    /// Commit changes
    pub async fn commit(&self, message: String) -> Result<GitCommitInfo> {
        self.inner.commit(message).await
    }
    
    /// Get commit history
    pub async fn get_commit_history(&self, branch_name: Option<String>, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        self.inner.get_commit_history(branch_name, limit).await
    }
    
    /// Get file history
    pub async fn get_file_history(&self, file_path: String, limit: Option<usize>) -> Result<Vec<GitCommitInfo>> {
        self.inner.get_file_history(file_path, limit).await
    }
    
    /// Get conflicts
    pub async fn get_conflicts(&self) -> Result<Vec<GitConflictInfo>> {
        self.inner.get_conflicts().await
    }
    
    /// Safe rollback to commit
    pub async fn safe_rollback_to_commit(&self, commit_id: String) -> Result<GitRollbackResult> {
        self.inner.safe_rollback_to_commit(commit_id).await
    }
    
    /// Get all available versions
    pub async fn get_all_versions(&self) -> Result<Vec<String>> {
        self.inner.get_all_versions().await
    }
    
    /// Rollback to version
    pub async fn rollback_to_version(&mut self, version: String) -> Result<()> {
        self.inner.rollback_to_version(version).await
    }
    
    /// Commit PDF version
    pub async fn commit_pdf_version(&self, pdf_path: String, latex_content: String) -> Result<GitCommitInfo> {
        self.inner.commit_pdf_version(pdf_path, latex_content).await
    }
}

impl Default for UnifiedGitTransport {
    fn default() -> Self {
        Self::new()
    }
}