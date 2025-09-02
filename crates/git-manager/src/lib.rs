use anyhow::Result;
use git2::{
    BranchType, Repository, Status, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

pub mod session;
pub mod operations;
pub mod history;
pub mod conflicts;

pub use session::SessionManager;
pub use operations::GitOperations;
pub use history::{CommitInfo, HistoryViewer, BranchInfo};
pub use conflicts::{ConflictResolver, ConflictInfo, ConflictType, Resolution, ResolutionChoice, CleanupOptions, RollbackResult};

#[derive(Clone, Debug)]
pub struct GitRepository {
    repo_path: PathBuf,
    session_branch: Option<String>,
    base_branch: String,
    // Repository is not Clone/Debug, so we manage it carefully
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitStatus {
    pub current_branch: String,
    pub session_branch: Option<String>,
    pub has_changes: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileStatus {
    pub path: String,
    pub status: GitFileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GitFileStatus {
    Untracked,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    UpdatedButUnmerged,
    Ignored,
}

impl GitRepository {
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        let base_branch = if repo_path.join(".git").exists() {
            let repo = Repository::open(&repo_path)?;
            Self::get_default_branch(&repo)?.unwrap_or_else(|| "main".to_string())
        } else {
            "main".to_string()
        };

        Ok(Self {
            repo_path,
            session_branch: None,
            base_branch,
        })
    }

    pub fn init_repository(&mut self) -> Result<()> {
        if !self.repo_path.join(".git").exists() {
            Repository::init(&self.repo_path)?;
            info!("Initialized new Git repository at {:?}", self.repo_path);
        }
        Ok(())
    }

    pub fn is_repository(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    pub fn get_status(&self) -> Result<GitStatus> {
        let repo = self.get_repository()?;

        let head = repo.head()?;
        let current_branch = if head.is_branch() {
            head.shorthand().unwrap_or("HEAD").to_string()
        } else {
            "HEAD".to_string()
        };

        let mut staged_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut untracked_files = Vec::new();

        let mut status_opts = StatusOptions::new();
        status_opts.include_untracked(true);
        status_opts.include_ignored(false);

        let statuses = repo.statuses(Some(&mut status_opts))?;
        let mut has_changes = false;

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            if status.contains(Status::INDEX_NEW)
                || status.contains(Status::INDEX_MODIFIED)
                || status.contains(Status::INDEX_DELETED)
                || status.contains(Status::INDEX_RENAMED)
                || status.contains(Status::INDEX_TYPECHANGE)
            {
                staged_files.push(path.clone());
                has_changes = true;
            }

            if status.contains(Status::WT_MODIFIED)
                || status.contains(Status::WT_DELETED)
                || status.contains(Status::WT_TYPECHANGE)
                || status.contains(Status::WT_RENAMED)
            {
                modified_files.push(path.clone());
                has_changes = true;
            }

            if status.contains(Status::WT_NEW) {
                untracked_files.push(path);
                has_changes = true;
            }
        }

        // Calculate commits ahead/behind (simplified for now)
        let commits_ahead = 0;
        let commits_behind = 0;

        Ok(GitStatus {
            current_branch,
            session_branch: self.session_branch.clone(),
            has_changes,
            staged_files,
            modified_files,
            untracked_files,
            commits_ahead,
            commits_behind,
        })
    }

    pub fn get_file_statuses(&self) -> Result<Vec<FileStatus>> {
        let repo = self.get_repository()?;
        let mut result = Vec::new();

        let mut status_opts = StatusOptions::new();
        status_opts.include_untracked(true);
        status_opts.include_ignored(false);

        let statuses = repo.statuses(Some(&mut status_opts))?;

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = Self::convert_git_status(entry.status());

            result.push(FileStatus { path, status });
        }

        Ok(result)
    }

    fn convert_git_status(status: Status) -> GitFileStatus {
        if status.contains(Status::WT_NEW) {
            GitFileStatus::Untracked
        } else if status.contains(Status::WT_MODIFIED) || status.contains(Status::INDEX_MODIFIED) {
            GitFileStatus::Modified
        } else if status.contains(Status::INDEX_NEW) {
            GitFileStatus::Added
        } else if status.contains(Status::WT_DELETED) || status.contains(Status::INDEX_DELETED) {
            GitFileStatus::Deleted
        } else if status.contains(Status::WT_RENAMED) || status.contains(Status::INDEX_RENAMED) {
            GitFileStatus::Renamed
        } else if status.contains(Status::IGNORED) {
            GitFileStatus::Ignored
        } else {
            GitFileStatus::Modified
        }
    }

    fn get_default_branch(repo: &Repository) -> Result<Option<String>> {
        // Try to get the default branch from remote
        if let Ok(remote) = repo.find_remote("origin") {
            if let Ok(default_branch) = remote.default_branch() {
                if let Some(branch_name) = default_branch.as_str() {
                    // Remove refs/heads/ prefix
                    let branch_name = branch_name.strip_prefix("refs/heads/").unwrap_or(branch_name);
                    return Ok(Some(branch_name.to_string()));
                }
            }
        }

        // Fallback: try common branch names
        let common_branches = ["main", "master", "develop"];
        for branch_name in &common_branches {
            if let Ok(_) = repo.find_branch(branch_name, BranchType::Local) {
                return Ok(Some(branch_name.to_string()));
            }
        }

        Ok(None)
    }

    pub fn set_base_branch(&mut self, branch_name: String) {
        self.base_branch = branch_name;
    }

    pub fn get_base_branch(&self) -> &str {
        &self.base_branch
    }

    pub fn get_session_branch(&self) -> Option<&str> {
        self.session_branch.as_deref()
    }

    pub fn get_repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn get_repository(&self) -> Result<Repository> {
        Ok(Repository::open(&self.repo_path)?)
    }
}