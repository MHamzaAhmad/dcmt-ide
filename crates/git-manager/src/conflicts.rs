use anyhow::{anyhow, Result};
use git2::{Index, IndexEntry, Repository, ResetType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::GitRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub file_path: String,
    pub conflict_type: ConflictType,
    pub our_content: Option<String>,
    pub their_content: Option<String>,
    pub base_content: Option<String>,
    pub merged_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    Content,
    ModifyDelete,
    DeleteModify,
    AddAdd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionChoice {
    pub file_path: String,
    pub resolution: Resolution,
    pub custom_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    TakeOurs,
    TakeTheirs,
    TakeBase,
    Custom,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupOptions {
    pub discard_unstaged: bool,
    pub discard_staged: bool,
    pub discard_untracked: bool,
    pub create_backup: bool,
}

pub struct ConflictResolver {
    git_repo: GitRepository,
}

impl ConflictResolver {
    pub fn new(git_repo: GitRepository) -> Self {
        Self { git_repo }
    }

    pub fn has_conflicts(&self) -> Result<bool> {
        let repo = self.git_repo.get_repository()?;

        let index = repo.index()?;
        Ok(index.has_conflicts())
    }

    pub fn get_conflicts(&self) -> Result<Vec<ConflictInfo>> {
        let repo = self.git_repo.get_repository()?;

        let index = repo.index()?;
        if !index.has_conflicts() {
            return Ok(Vec::new());
        }

        let mut conflicts = Vec::new();
        let conflict_iter = index.conflicts()?;

        for conflict in conflict_iter {
            let conflict = conflict?;
            let file_path = self.get_conflict_path(&conflict)?;
            
            let conflict_info = self.analyze_conflict(&conflict, &file_path, &repo)?;
            conflicts.push(conflict_info);
        }

        Ok(conflicts)
    }

    pub fn resolve_conflicts(&self, resolutions: Vec<ResolutionChoice>) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let mut index = repo.index()?;

        for resolution in resolutions {
            match resolution.resolution {
                Resolution::TakeOurs => {
                    self.resolve_take_ours(&mut index, &resolution.file_path)?;
                }
                Resolution::TakeTheirs => {
                    self.resolve_take_theirs(&mut index, &resolution.file_path)?;
                }
                Resolution::TakeBase => {
                    self.resolve_take_base(&mut index, &resolution.file_path)?;
                }
                Resolution::Custom => {
                    if let Some(content) = resolution.custom_content {
                        self.resolve_with_content(&mut index, &resolution.file_path, &content)?;
                    } else {
                        return Err(anyhow!("Custom resolution requires content"));
                    }
                }
                Resolution::Manual => {
                    // File should already be resolved manually by user
                    self.mark_resolved(&mut index, &resolution.file_path)?;
                }
            }
        }

        index.write()?;
        Ok(())
    }

    pub fn abort_merge(&self) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Reset to HEAD to abort merge
        let head = repo.head()?.peel_to_commit()?;
        repo.reset(&head.into_object(), ResetType::Hard, None)?;
        repo.cleanup_state()?;

        Ok(())
    }

    pub fn check_uncommitted_changes(&self) -> Result<bool> {
        let status = self.git_repo.get_status()?;
        Ok(status.has_changes)
    }

    pub fn get_cleanup_preview(&self, options: &CleanupOptions) -> Result<Vec<String>> {
        let mut files_to_clean = Vec::new();
        let status = self.git_repo.get_status()?;

        if options.discard_unstaged {
            files_to_clean.extend(status.modified_files.iter().cloned());
        }

        if options.discard_untracked {
            files_to_clean.extend(status.untracked_files.iter().cloned());
        }

        if options.discard_staged {
            files_to_clean.extend(status.staged_files.iter().cloned());
        }

        Ok(files_to_clean)
    }

    pub fn cleanup_workspace(&self, options: CleanupOptions) -> Result<Vec<String>> {
        let repo = self.git_repo.get_repository()?;

        let mut cleaned_files = Vec::new();

        if options.create_backup {
            self.create_backup()?;
        }

        let status = self.git_repo.get_status()?;

        // Discard unstaged changes
        if options.discard_unstaged {
            let head = repo.head()?.peel_to_commit()?;
            let head_tree = head.tree()?;

            for file_path in &status.modified_files {
                let mut checkout_builder = git2::build::CheckoutBuilder::new();
                checkout_builder.path(file_path);
                checkout_builder.force();
                repo.checkout_tree(head_tree.as_object(), Some(&mut checkout_builder))?;
                cleaned_files.push(file_path.clone());
            }
        }

        // Remove untracked files
        if options.discard_untracked {
            for file_path in &status.untracked_files {
                let full_path = self.git_repo.get_repo_path().join(file_path);
                if full_path.exists() {
                    if full_path.is_dir() {
                        fs::remove_dir_all(&full_path)?;
                    } else {
                        fs::remove_file(&full_path)?;
                    }
                    cleaned_files.push(file_path.clone());
                }
            }
        }

        // Reset staged changes
        if options.discard_staged {
            let head = repo.head()?.peel_to_commit()?;
            repo.reset(&head.into_object(), ResetType::Mixed, None)?;
            cleaned_files.extend(status.staged_files);
        }

        Ok(cleaned_files)
    }

    pub fn safe_rollback_to_commit(&self, commit_id: &str) -> Result<RollbackResult> {
        let repo = self.git_repo.get_repository()?;

        // Check for uncommitted changes
        let has_changes = self.check_uncommitted_changes()?;
        if has_changes {
            return Ok(RollbackResult::HasUncommittedChanges(
                self.git_repo.get_status()?
            ));
        }

        // Verify commit exists
        let oid = git2::Oid::from_str(commit_id)?;
        let target_commit = repo.find_commit(oid)?;

        // Get commit message before reset
        let commit_message = target_commit.message().unwrap_or("").to_string();

        // Perform reset
        repo.reset(&target_commit.into_object(), ResetType::Hard, None)?;

        Ok(RollbackResult::Success {
            commit_id: commit_id.to_string(),
            commit_message,
        })
    }

    fn analyze_conflict(&self, conflict: &git2::IndexConflict, file_path: &str, repo: &Repository) -> Result<ConflictInfo> {
        let conflict_type = if conflict.our.is_some() && conflict.their.is_some() {
            ConflictType::Content
        } else if conflict.our.is_some() && conflict.their.is_none() {
            ConflictType::DeleteModify
        } else if conflict.our.is_none() && conflict.their.is_some() {
            ConflictType::ModifyDelete
        } else {
            ConflictType::AddAdd
        };

        let our_content = self.get_blob_content(repo, &conflict.our)?;
        let their_content = self.get_blob_content(repo, &conflict.their)?;
        let base_content = self.get_blob_content(repo, &conflict.ancestor)?;

        // Read the current (merged) content from working directory
        let merged_content = self.read_file_content(file_path).ok();

        Ok(ConflictInfo {
            file_path: file_path.to_string(),
            conflict_type,
            our_content,
            their_content,
            base_content,
            merged_content,
        })
    }

    fn get_conflict_path(&self, conflict: &git2::IndexConflict) -> Result<String> {
        if let Some(ref entry) = conflict.our {
            Ok(String::from_utf8_lossy(&entry.path).to_string())
        } else if let Some(ref entry) = conflict.their {
            Ok(String::from_utf8_lossy(&entry.path).to_string())
        } else if let Some(ref entry) = conflict.ancestor {
            Ok(String::from_utf8_lossy(&entry.path).to_string())
        } else {
            Err(anyhow!("Could not determine conflict path"))
        }
    }

    fn get_blob_content(&self, repo: &Repository, entry: &Option<IndexEntry>) -> Result<Option<String>> {
        if let Some(entry) = entry {
            let blob = repo.find_blob(entry.id)?;
            let content = String::from_utf8_lossy(blob.content()).to_string();
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    fn read_file_content(&self, file_path: &str) -> Result<String> {
        let full_path = self.git_repo.get_repo_path().join(file_path);
        Ok(fs::read_to_string(full_path)?)
    }

    fn resolve_take_ours(&self, index: &mut Index, file_path: &str) -> Result<()> {
        // Remove conflict and add our version
        index.remove_path(Path::new(file_path))?;
        index.add_path(Path::new(file_path))?;
        Ok(())
    }

    fn resolve_take_theirs(&self, index: &mut Index, file_path: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Get their version and write it to working directory
        if let Ok(conflicts_iter) = index.conflicts() {
            for conflict in conflicts_iter {
                let conflict = conflict?;
                let conflict_path = self.get_conflict_path(&conflict)?;
                if conflict_path == file_path {
                    if let Some(ref their_entry) = conflict.their {
                        let blob = repo.find_blob(their_entry.id)?;
                        let full_path = self.git_repo.get_repo_path().join(file_path);
                        fs::write(full_path, blob.content())?;
                        break;
                    }
                }
            }
        }

        index.remove_path(Path::new(file_path))?;
        index.add_path(Path::new(file_path))?;
        Ok(())
    }

    fn resolve_take_base(&self, index: &mut Index, file_path: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Get base version and write it to working directory
        if let Ok(conflicts_iter) = index.conflicts() {
            for conflict in conflicts_iter {
                let conflict = conflict?;
                let conflict_path = self.get_conflict_path(&conflict)?;
                if conflict_path == file_path {
                    if let Some(ref base_entry) = conflict.ancestor {
                        let blob = repo.find_blob(base_entry.id)?;
                        let full_path = self.git_repo.get_repo_path().join(file_path);
                        fs::write(full_path, blob.content())?;
                        break;
                    }
                }
            }
        }

        index.remove_path(Path::new(file_path))?;
        index.add_path(Path::new(file_path))?;
        Ok(())
    }

    fn resolve_with_content(&self, index: &mut Index, file_path: &str, content: &str) -> Result<()> {
        let full_path = self.git_repo.get_repo_path().join(file_path);
        fs::write(full_path, content)?;

        index.remove_path(Path::new(file_path))?;
        index.add_path(Path::new(file_path))?;
        Ok(())
    }

    fn mark_resolved(&self, index: &mut Index, file_path: &str) -> Result<()> {
        index.remove_path(Path::new(file_path))?;
        index.add_path(Path::new(file_path))?;
        Ok(())
    }

    fn create_backup(&self) -> Result<String> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = format!("backup_{}", timestamp);
        let backup_path = self.git_repo.get_repo_path().join(&backup_dir);
        
        // Create backup directory (simplified implementation)
        fs::create_dir_all(&backup_path)?;
        
        // In a real implementation, you'd copy the working directory
        // to the backup location
        
        Ok(backup_dir)
    }

    pub fn get_git_repository(&self) -> &GitRepository {
        &self.git_repo
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackResult {
    Success {
        commit_id: String,
        commit_message: String,
    },
    HasUncommittedChanges(crate::GitStatus),
    ConflictsDetected(Vec<ConflictInfo>),
}