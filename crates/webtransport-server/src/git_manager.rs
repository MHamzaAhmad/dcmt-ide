use std::path::PathBuf;
use anyhow::Result;

use crate::types::{GitOp, GitResponseData, GitStatusResponse, GitCommitInfo, GitRollbackResult, GitConflictInfo, GitConflictType};

// Import Git manager components when feature is enabled
#[cfg(feature = "native-git")]
use latex_ide_git_manager::{GitRepository, SessionManager, GitOperations, HistoryViewer, ConflictResolver};

/// Git server manager that wraps all Git operations for a workspace
pub struct GitServerManager {
    #[cfg(feature = "native-git")]
    pub git_repo: GitRepository,
    #[cfg(feature = "native-git")]
    pub session_manager: Option<SessionManager>,
    #[cfg(feature = "native-git")]
    pub git_operations: GitOperations,
    #[cfg(feature = "native-git")]
    pub history_viewer: HistoryViewer,
    #[cfg(feature = "native-git")]
    pub conflict_resolver: ConflictResolver,
    
    // Fallback for when Git is disabled
    #[cfg(not(feature = "native-git"))]
    workspace_path: PathBuf,
}

impl GitServerManager {
    #[cfg(feature = "native-git")]
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
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
    
    #[cfg(not(feature = "native-git"))]
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        Ok(Self { workspace_path })
    }
    
    #[cfg(feature = "native-git")]
    pub fn dummy(workspace_path: PathBuf) -> Self {
        let git_repo = GitRepository::new(workspace_path).unwrap_or_else(|_| 
            GitRepository::new(PathBuf::from(".")).unwrap()
        );
        let git_operations = GitOperations::new(git_repo.clone());
        let history_viewer = HistoryViewer::new(git_repo.clone());
        let conflict_resolver = ConflictResolver::new(git_repo.clone());
        
        Self {
            git_repo,
            session_manager: None,
            git_operations,
            history_viewer,
            conflict_resolver,
        }
    }
    
    #[cfg(not(feature = "native-git"))]
    pub fn dummy(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    pub async fn execute_operation(&mut self, operation: GitOp) -> Result<Option<GitResponseData>> {
        #[cfg(feature = "native-git")]
        {
            self.execute_operation_native(operation).await
        }
        
        #[cfg(not(feature = "native-git"))]
        {
            let _ = operation; // Silence unused variable warning
            Err(anyhow::anyhow!("Git operations not available in this build"))
        }
    }
    
    #[cfg(feature = "native-git")]
    async fn execute_operation_native(&mut self, operation: GitOp) -> Result<Option<GitResponseData>> {
        match operation {
            GitOp::InitRepository { path: _ } => {
                let status = self.git_repo.get_status()?;
                Ok(Some(GitResponseData::Status(convert_git_status(status))))
            }
            
            GitOp::GetStatus => {
                let status = self.git_repo.get_status()?;
                Ok(Some(GitResponseData::Status(convert_git_status(status))))
            }
            
            GitOp::GetBranches => {
                let branches = self.git_operations.list_branches()?;
                Ok(Some(GitResponseData::Branches(branches)))
            }
            
            GitOp::GetCurrentBranch => {
                let current = self.git_operations.get_current_branch()?;
                Ok(Some(GitResponseData::CurrentBranch(current)))
            }
            
            GitOp::StartSession => {
                if let Some(ref mut session_mgr) = self.session_manager {
                    let session_branch = session_mgr.start_session()?;
                    Ok(Some(GitResponseData::SessionBranch(session_branch)))
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::EndSession { save_changes } => {
                if let Some(ref mut session_mgr) = self.session_manager {
                    session_mgr.end_session(save_changes)?;
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("Session manager not available"))
                }
            }
            
            GitOp::StageFile { file_path } => {
                self.git_operations.stage_file(&file_path)?;
                Ok(None)
            }
            
            GitOp::UnstageFile { file_path } => {
                self.git_operations.unstage_file(&file_path)?;
                Ok(None)
            }
            
            GitOp::StageAllChanges => {
                self.git_operations.stage_all_changes()?;
                Ok(None)
            }
            
            GitOp::Commit { message } => {
                if let Some(ref session_mgr) = self.session_manager {
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
                if let Some(ref mut session_mgr) = self.session_manager {
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
                let commits = self.history_viewer.get_commit_history(
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
                let result = self.git_operations.safe_rollback_to_commit(&commit_id)?;
                match result {
                    latex_ide_git_manager::RollbackResult::Success { commit_id, commit_message } => {
                        Ok(Some(GitResponseData::RollbackResult(GitRollbackResult::Success {
                            commit_id,
                            commit_message,
                        })))
                    }
                    latex_ide_git_manager::RollbackResult::HasUncommittedChanges(status) => {
                        Ok(Some(GitResponseData::RollbackResult(GitRollbackResult::HasUncommittedChanges(
                            convert_git_status(status)
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
                self.git_operations.create_branch(&branch_name, from_current)?;
                Ok(None)
            }
            
            GitOp::CheckoutBranch { branch_name } => {
                self.git_operations.checkout_branch(&branch_name)?;
                Ok(None)
            }
            
            GitOp::DeleteBranch { branch_name, force } => {
                self.git_operations.delete_branch(&branch_name, force)?;
                Ok(None)
            }
            
            GitOp::DiscardFileChanges { file_path } => {
                self.git_operations.discard_file_changes(&file_path)?;
                Ok(None)
            }
            
            GitOp::DiscardAllChanges => {
                self.git_operations.discard_all_changes()?;
                Ok(None)
            }
            
            GitOp::GetConflicts => {
                let conflicts = self.conflict_resolver.get_current_conflicts()?;
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
                            crate::types::GitResolution::TakeOurs => latex_ide_git_manager::Resolution::TakeOurs,
                            crate::types::GitResolution::TakeTheirs => latex_ide_git_manager::Resolution::TakeTheirs,
                            crate::types::GitResolution::TakeBase => latex_ide_git_manager::Resolution::TakeBase,
                            crate::types::GitResolution::Custom => latex_ide_git_manager::Resolution::Custom,
                            crate::types::GitResolution::Manual => latex_ide_git_manager::Resolution::Manual,
                        },
                        custom_content: res.custom_content,
                    })
                    .collect();
                self.conflict_resolver.resolve_conflicts(git_resolutions)?;
                Ok(None)
            }
            
            GitOp::GetFileHistory { file_path, limit } => {
                let commits = self.history_viewer.get_file_history(&file_path, limit.unwrap_or(20))?;
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
                Err(anyhow::anyhow!("Remote operations not yet implemented"))
            }
            
            GitOp::PullFromRemote { remote_name: _, branch_name: _ } => {
                Err(anyhow::anyhow!("Remote operations not yet implemented"))
            }
            
            // Operations that don't need implementation
            GitOp::SearchCommits { query: _, limit: _ } => {
                Err(anyhow::anyhow!("Search commits not yet implemented"))
            }
            
            GitOp::GetCommitDetails { commit_id: _ } => {
                Err(anyhow::anyhow!("Get commit details not yet implemented"))
            }
            
            GitOp::MergeBranch { branch_name: _, message: _ } => {
                Err(anyhow::anyhow!("Merge branch not yet implemented"))
            }
            
            GitOp::AbortMerge => {
                Err(anyhow::anyhow!("Abort merge not yet implemented"))
            }
        }
    }
}

#[cfg(feature = "native-git")]
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