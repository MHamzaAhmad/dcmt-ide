use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use git2::{BranchType, Commit, Oid, Repository};
use serde::{Deserialize, Serialize};
use crate::GitRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
    pub is_merge: bool,
    pub files_changed: Vec<String>,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub last_commit: CommitInfo,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDiff {
    pub file_path: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub diff_lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    Header,
}

pub struct HistoryViewer {
    git_repo: GitRepository,
}

impl HistoryViewer {
    pub fn new(git_repo: GitRepository) -> Self {
        Self { git_repo }
    }

    pub fn get_commit_history(&self, branch_name: Option<&str>, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let repo = self.git_repo.get_repository()?;

        let mut revwalk = repo.revwalk()?;
        
        if let Some(branch_name) = branch_name {
            let branch = repo.find_branch(branch_name, BranchType::Local)?;
            revwalk.push(branch.get().target().unwrap())?;
        } else {
            revwalk.push_head()?;
        }

        let mut commits = Vec::new();
        let mut count = 0;

        for oid_result in revwalk {
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }
            
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            let commit_info = self.convert_commit_to_info(&commit, &repo)?;
            
            commits.push(commit_info);
            count += 1;
        }

        Ok(commits)
    }

    pub fn get_branch_history(&self, branch_name: &str, base_branch: Option<&str>) -> Result<Vec<CommitInfo>> {
        let repo = self.git_repo.get_repository()?;

        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let branch_commit = branch.get().peel_to_commit()?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(branch_commit.id())?;

        // If base branch is specified, hide commits that are in base branch
        if let Some(base_branch_name) = base_branch {
            let base_branch = repo.find_branch(base_branch_name, BranchType::Local)?;
            let base_commit = base_branch.get().peel_to_commit()?;
            revwalk.hide(base_commit.id())?;
        }

        let mut commits = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            let commit_info = self.convert_commit_to_info(&commit, &repo)?;
            commits.push(commit_info);
        }

        Ok(commits)
    }

    pub fn get_commit_details(&self, commit_id: &str) -> Result<CommitInfo> {
        let repo = self.git_repo.get_repository()?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        
        self.convert_commit_to_info(&commit, &repo)
    }

    pub fn get_commit_diff(&self, commit_id: &str) -> Result<Vec<CommitDiff>> {
        let repo = self.git_repo.get_repository()?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        
        let commit_tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            None,
        )?;

        let commit_diffs = Vec::new();

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, _line| {
            // For now, we'll implement a basic diff parser
            // In a more complete implementation, you'd want to build proper diff structures
            true
        })?;

        // This is a simplified implementation
        // In a real implementation, you'd want to properly parse the diff
        Ok(commit_diffs)
    }

    pub fn get_branches_info(&self) -> Result<Vec<BranchInfo>> {
        let repo = self.git_repo.get_repository()?;

        let mut branches_info = Vec::new();
        let branches = repo.branches(Some(BranchType::Local))?;
        let current_branch = self.get_current_branch_name(&repo)?;

        for branch_result in branches {
            let (branch, _) = branch_result?;
            if let Some(branch_name) = branch.name()? {
                let branch_commit = branch.get().peel_to_commit()?;
                let commit_info = self.convert_commit_to_info(&branch_commit, &repo)?;
                
                let is_current = Some(branch_name) == current_branch.as_deref();
                
                // Calculate ahead/behind (simplified implementation)
                let ahead = 0; // Would need to implement proper ahead/behind calculation
                let behind = 0;

                let branch_info = BranchInfo {
                    name: branch_name.to_string(),
                    is_current,
                    last_commit: commit_info,
                    ahead,
                    behind,
                };

                branches_info.push(branch_info);
            }
        }

        Ok(branches_info)
    }

    pub fn get_file_history(&self, file_path: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let repo = self.git_repo.get_repository()?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        let mut count = 0;

        for oid_result in revwalk {
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            
            // Check if this commit affects the file
            if self.commit_affects_file(&commit, file_path, &repo)? {
                let commit_info = self.convert_commit_to_info(&commit, &repo)?;
                commits.push(commit_info);
                count += 1;
            }
        }

        Ok(commits)
    }

    pub fn search_commits(&self, query: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let repo = self.git_repo.get_repository()?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        let mut count = 0;

        let query_lower = query.to_lowercase();

        for oid_result in revwalk {
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }

            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            
            // Search in commit message and author
            let message = commit.message().unwrap_or("").to_lowercase();
            let author = commit.author().name().unwrap_or("").to_lowercase();
            
            if message.contains(&query_lower) || author.contains(&query_lower) {
                let commit_info = self.convert_commit_to_info(&commit, &repo)?;
                commits.push(commit_info);
                count += 1;
            }
        }

        Ok(commits)
    }

    fn convert_commit_to_info(&self, commit: &Commit, repo: &Repository) -> Result<CommitInfo> {
        let id = commit.id().to_string();
        let short_id = format!("{:.7}", id);
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        
        // Convert Git time to DateTime<Utc>
        let time = author.when();
        let timestamp = Utc.timestamp_opt(time.seconds(), 0).single()
            .unwrap_or_else(|| Utc::now());

        let parents: Vec<String> = (0..commit.parent_count())
            .map(|i| commit.parent_id(i).unwrap().to_string())
            .collect();

        let is_merge = commit.parent_count() > 1;

        // Get stats (simplified)
        let mut files_changed = Vec::new();
        let insertions = 0;
        let deletions = 0;

        // Get changed files (simplified implementation)
        if let Ok(commit_tree) = commit.tree() {
            if let Ok(parent) = commit.parent(0) {
                if let Ok(parent_tree) = parent.tree() {
                    if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None) {
                        diff.print(git2::DiffFormat::NameStatus, |delta, _hunk, _line| {
                            if let Some(new_file) = delta.new_file().path() {
                                if let Some(path_str) = new_file.to_str() {
                                    files_changed.push(path_str.to_string());
                                }
                            }
                            true
                        }).ok();
                    }
                }
            } else {
                // Initial commit - all files are new
                // This is a simplified approach
            }
        }

        Ok(CommitInfo {
            id,
            short_id,
            message,
            author_name,
            author_email,
            timestamp,
            parents,
            is_merge,
            files_changed,
            insertions,
            deletions,
        })
    }

    fn get_current_branch_name(&self, repo: &Repository) -> Result<Option<String>> {
        match repo.head() {
            Ok(head) => {
                if head.is_branch() {
                    Ok(head.shorthand().map(|s| s.to_string()))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None)
        }
    }

    fn commit_affects_file(&self, commit: &Commit, file_path: &str, repo: &Repository) -> Result<bool> {
        let commit_tree = commit.tree()?;
        
        if commit.parent_count() == 0 {
            // Initial commit - check if file exists in this commit
            return Ok(commit_tree.get_path(std::path::Path::new(file_path)).is_ok());
        }

        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
        
        let mut affects_file = false;
        diff.print(git2::DiffFormat::NameStatus, |delta, _hunk, _line| {
            if let Some(new_file) = delta.new_file().path() {
                if new_file.to_str() == Some(file_path) {
                    affects_file = true;
                }
            }
            if let Some(old_file) = delta.old_file().path() {
                if old_file.to_str() == Some(file_path) {
                    affects_file = true;
                }
            }
            true
        })?;

        Ok(affects_file)
    }

    pub fn get_git_repository(&self) -> &GitRepository {
        &self.git_repo
    }
}