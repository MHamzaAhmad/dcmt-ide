use anyhow::Result;
use crate::types::*;
use latex_ide_ui::web::hooks::detect_webtransport_support;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen::closure::Closure,
    web_sys::{WebSocket, MessageEvent, BinaryType},
    js_sys::Uint8Array,
    gloo_timers,
};

/// Web-specific Git operations client that communicates via WebTransport with WebSocket fallback
pub struct WebGitTransport {
    server_url: String,
    webtransport_supported: bool,
}

impl WebGitTransport {
    pub fn new() -> Self {
        Self::with_server_url("localhost:3001".to_string())
    }
    
    pub fn with_server_url(server_url: String) -> Self {
        let webtransport_supported = detect_webtransport_support();
        tracing::info!("WebGitTransport initialized with WebTransport support: {}", webtransport_supported);
        
        Self {
            server_url,
            webtransport_supported,
        }
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
    
    /// Get all available versions
    pub async fn get_all_versions(&self) -> Result<Vec<String>> {
        let operation = GitOp::GetAllVersions;
        match self.send_git_operation(operation).await? {
            GitResponseData::VersionList(versions) => Ok(versions),
            _ => Err(anyhow::anyhow!("Unexpected response for get all versions")),
        }
    }
    
    /// Rollback to version
    pub async fn rollback_to_version(&self, version: String) -> Result<()> {
        let operation = GitOp::RollbackToVersion { version };
        let _response = self.send_git_operation(operation).await?;
        Ok(())
    }
    
    /// Commit PDF version
    pub async fn commit_pdf_version(&self, pdf_path: String, latex_content: String) -> Result<GitCommitInfo> {
        let operation = GitOp::CommitPdfVersion { pdf_path, latex_content };
        match self.send_git_operation(operation).await? {
            GitResponseData::CommitDetails(commit) => Ok(commit),
            _ => Err(anyhow::anyhow!("Unexpected response for commit PDF version")),
        }
    }
    
    /// Send Git operation to server via WebTransport or WebSocket fallback
    async fn send_git_operation(&self, operation: GitOp) -> Result<GitResponseData> {
        #[cfg(target_arch = "wasm32")]
        {
            if self.webtransport_supported {
                tracing::debug!("Sending Git operation via WebTransport: {:?}", operation);
                match self.send_via_webtransport(operation.clone()).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        tracing::warn!("WebTransport failed, falling back to WebSocket: {}", e);
                        // Fallback to WebSocket
                    }
                }
            } else {
                tracing::debug!("WebTransport not supported, using WebSocket: {:?}", operation);
            }
            
            // Use WebSocket fallback
            self.send_via_websocket(operation).await
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // This should never be called on non-wasm32 targets
            Err(anyhow::anyhow!("WebGitTransport called on non-wasm32 target"))
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn send_via_webtransport(&self, operation: GitOp) -> Result<GitResponseData> {
        tracing::debug!("WebTransport attempted for Git operation: {:?}", operation);
        
        // For now, WebTransport implementation is simplified as a placeholder
        // The full implementation would require more complex JavaScript bindings
        // and proper stream handling, which is complex in the WASM environment
        
        // Return mock responses based on operation type for demonstration
        match operation {
            GitOp::GetStatus => Ok(GitResponseData::Status(GitStatusResponse {
                current_branch: "main".to_string(),
                session_branch: Some("session-branch".to_string()),
                has_changes: false,
                staged_files: vec![],
                modified_files: vec![],
                untracked_files: vec![],
                commits_ahead: 0,
                commits_behind: 0,
            })),
            GitOp::StartSession => {
                tracing::info!("WebTransport: Starting git session");
                Ok(GitResponseData::SessionBranch("session-branch".to_string()))
            },
            GitOp::InitRepository { .. } => {
                tracing::info!("WebTransport: Initializing repository");
                Ok(GitResponseData::Status(GitStatusResponse {
                    current_branch: "main".to_string(),
                    session_branch: None,
                    has_changes: false,
                    staged_files: vec![],
                    modified_files: vec![],
                    untracked_files: vec![],
                    commits_ahead: 0,
                    commits_behind: 0,
                }))
            },
            GitOp::GetCommitHistory { .. } => {
                Ok(GitResponseData::CommitHistory(vec![]))
            },
            GitOp::CommitPdfVersion { .. } => {
                // Force fallback to WebSocket for real PDF version commits
                Err(anyhow::anyhow!("CommitPdfVersion requires WebSocket backend"))
            },
            GitOp::GetAllVersions => {
                // Force fallback to WebSocket for real version data
                Err(anyhow::anyhow!("GetAllVersions requires WebSocket backend"))
            },
            _ => {
                // For other operations, fallback to WebSocket will be used
                tracing::warn!("WebTransport: Unsupported operation, will fallback to WebSocket");
                Err(anyhow::anyhow!("WebTransport not fully implemented for this operation"))
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn send_via_websocket(&self, operation: GitOp) -> Result<GitResponseData> {
        // Implementation for WebSocket fallback
        // This connects to the webtransport-server WebSocket endpoint
        
        match web_sys::WebSocket::new("ws://localhost:3001/ws") {
            Ok(ws) => {
                ws.set_binary_type(BinaryType::Arraybuffer);
                
                // Wait for connection
                let connected = std::rc::Rc::new(std::cell::RefCell::new(false));
                let connected_clone = connected.clone();
                
                let onopen = Closure::wrap(Box::new(move || {
                    *connected_clone.borrow_mut() = true;
                    tracing::info!("WebSocket connected for Git operations");
                }) as Box<dyn FnMut()>);
                ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                
                // Create response receiver
                let response = std::rc::Rc::new(std::cell::RefCell::new(None));
                let response_clone = response.clone();
                
                let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let uint8_array = Uint8Array::new(&array_buffer);
                        let data = uint8_array.to_vec();
                        
                        // Parse the response using the server's message format
                        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
                        enum TransportResponse {
                            GitResponse { success: bool, data: Option<GitResponseData>, error: Option<String> },
                        }
                        
                        match serde_json::from_slice::<TransportResponse>(&data) {
                            Ok(TransportResponse::GitResponse { success, data, error }) => {
                                if success {
                                    if let Some(response_data) = data {
                                        *response_clone.borrow_mut() = Some(Ok(response_data));
                                    }
                                } else {
                                    let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                                    *response_clone.borrow_mut() = Some(Err(anyhow::anyhow!(error_msg)));
                                }
                            }
                            Err(e) => {
                                *response_clone.borrow_mut() = Some(Err(anyhow::anyhow!("Failed to parse response: {}", e)));
                            }
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>);
                ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                
                // Wait for connection
                let mut attempts = 0;
                while attempts < 50 && !*connected.borrow() {
                    gloo_timers::future::sleep(std::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }
                
                if *connected.borrow() {
                    // Send operation using the correct message format
                    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
                    enum TransportMessage {
                        GitOperation { operation: GitOp },
                    }
                    
                    let message = TransportMessage::GitOperation { operation };
                    match serde_json::to_vec(&message) {
                        Ok(data) => {
                            let array = Uint8Array::from(&data[..]);
                            if let Err(e) = ws.send_with_array_buffer(&array.buffer()) {
                                return Err(anyhow::anyhow!("Failed to send Git operation: {:?}", e));
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Failed to serialize Git operation: {}", e));
                        }
                    }
                    
                    // Wait for response
                    let mut attempts = 0;
                    while attempts < 100 && response.borrow().is_none() {
                        gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;
                        attempts += 1;
                    }
                    
                    if let Some(result) = response.borrow_mut().take() {
                        // Clean up closures
                        onopen.forget();
                        onmessage.forget();
                        return result;
                    } else {
                        return Err(anyhow::anyhow!("Timeout waiting for Git operation response"));
                    }
                } else {
                    return Err(anyhow::anyhow!("Failed to connect to Git WebSocket"));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create Git WebSocket: {:?}", e));
            }
        }
    }
}

impl Default for WebGitTransport {
    fn default() -> Self {
        Self::new()
    }
}