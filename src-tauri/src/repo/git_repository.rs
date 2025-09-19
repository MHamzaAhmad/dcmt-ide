use anyhow::{Context, Result};
use git2::{
    BranchType, Delta, DiffOptions, ErrorCode, Repository, RepositoryOpenFlags,
    Signature, Status, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<GitFileStatus>,
    pub unstaged: Vec<GitFileStatus>,
    pub untracked: Vec<String>,
    pub is_initialized: bool,
    pub has_commits: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: FileChangeType,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

pub struct GitRepository {
    repo: Mutex<Repository>,
    workspace_path: PathBuf,
}

impl GitRepository {
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let repo = Repository::open_ext(
            &workspace_path,
            RepositoryOpenFlags::empty(),
            Vec::<&Path>::new(),
        )
        .with_context(|| format!("Failed to open git repository at {:?}", workspace_path))?;

        Ok(Self {
            repo: Mutex::new(repo),
            workspace_path,
        })
    }

    pub fn repository_exists(workspace_path: &Path) -> bool {
        Repository::open_ext(workspace_path, RepositoryOpenFlags::empty(), Vec::<&Path>::new()).is_ok()
    }

    #[allow(dead_code)]
    pub fn has_commits(&self) -> Result<bool> {
        let repo = self.repo.lock().unwrap();
        let has_commits = match repo.head() {
            Ok(_) => true,
            Err(e) if e.code() == ErrorCode::UnbornBranch => false,
            Err(e) => return Err(e.into()),
        };
        Ok(has_commits)
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let head = repo.head()?;
        
        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Ok("HEAD".to_string())
        }
    }

    pub fn get_status(&self) -> Result<GitStatus> {
        let repo = self.repo.lock().unwrap();

        // Check if repository has commits
        let has_commits = match repo.head() {
            Ok(_) => true,
            Err(e) if e.code() == ErrorCode::UnbornBranch => false,
            Err(e) => return Err(e.into()),
        };

        let branch = if has_commits {
            let head = repo.head()?;
            let branch_name = if let Some(name) = head.shorthand() {
                name.to_string()
            } else {
                "HEAD".to_string()
            };
            branch_name
        } else {
            // Default branch name when no commits
            "main".to_string()
        };

        // Get ahead/behind counts (only if we have commits and a valid branch)
        let (ahead, behind) = if has_commits {
            self.get_ahead_behind_internal(&repo, &branch).unwrap_or((0, 0))
        } else {
            (0, 0)
        };

        // Get file statuses
        let mut status_options = StatusOptions::new();
        status_options.include_untracked(true);

        let statuses = repo.statuses(Some(&mut status_options))?;

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            if status.contains(Status::WT_NEW) {
                untracked.push(path.clone());
            }

            // Check staged changes
            if status.intersects(
                Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED | Status::INDEX_RENAMED,
            ) {
                let change_type = if status.contains(Status::INDEX_NEW) {
                    FileChangeType::Added
                } else if status.contains(Status::INDEX_MODIFIED) {
                    FileChangeType::Modified
                } else if status.contains(Status::INDEX_DELETED) {
                    FileChangeType::Deleted
                } else {
                    FileChangeType::Renamed
                };

                staged.push(GitFileStatus {
                    path: path.clone(),
                    status: change_type,
                    additions: 0,
                    deletions: 0,
                });
            }

            // Check unstaged changes
            if status.intersects(
                Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_RENAMED,
            ) {
                let change_type = if status.contains(Status::WT_MODIFIED) {
                    FileChangeType::Modified
                } else if status.contains(Status::WT_DELETED) {
                    FileChangeType::Deleted
                } else {
                    FileChangeType::Renamed
                };

                unstaged.push(GitFileStatus {
                    path,
                    status: change_type,
                    additions: 0,
                    deletions: 0,
                });
            }
        }

        Ok(GitStatus {
            branch,
            ahead,
            behind,
            staged,
            unstaged,
            untracked,
            is_initialized: true,
            has_commits,
        })
    }

    pub fn get_diff(&self, staged: bool) -> Result<GitDiff> {
        let repo = self.repo.lock().unwrap();
        let mut diff_options = DiffOptions::new();
        
        let diff = if staged {
            // Get staged changes (index vs HEAD)
            let head = repo.head()?.peel_to_tree()?;
            let index = repo.index()?;
            repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_options))?
        } else {
            // Get unstaged changes (workdir vs index)
            let index = repo.index()?;
            repo.diff_index_to_workdir(Some(&index), Some(&mut diff_options))?
        };
        
        let files = Rc::new(RefCell::new(Vec::new()));
        let total_additions = Rc::new(RefCell::new(0usize));
        let total_deletions = Rc::new(RefCell::new(0usize));
        let current_file_index = Rc::new(RefCell::new(None::<usize>));
        
        {
            let files_clone = files.clone();
            let current_file_index_clone = current_file_index.clone();
            let total_additions_clone = total_additions.clone();
            let total_deletions_clone = total_deletions.clone();
            let files_for_line = files.clone();
            let current_file_index_for_line = current_file_index.clone();
            
            diff.foreach(
                &mut |delta, _progress| {
                    let path = delta.new_file().path()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let old_path = delta.old_file().path()
                        .and_then(|p| p.to_str())
                        .map(|s| s.to_string());
                    
                    let status = match delta.status() {
                        Delta::Added => FileChangeType::Added,
                        Delta::Deleted => FileChangeType::Deleted,
                        Delta::Modified => FileChangeType::Modified,
                        Delta::Renamed => FileChangeType::Renamed,
                        _ => FileChangeType::Modified,
                    };
                    
                    let mut files_ref = files_clone.borrow_mut();
                    files_ref.push(FileDiff {
                        path,
                        old_path,
                        status,
                        additions: 0,
                        deletions: 0,
                        hunks: Vec::new(),
                    });
                    
                    *current_file_index_clone.borrow_mut() = Some(files_ref.len() - 1);
                    
                    true
                },
                None,
                None,
                Some(&mut |_delta, _hunk, line| {
                    match line.origin() {
                        '+' => {
                            *total_additions_clone.borrow_mut() += 1;
                            if let Some(index) = *current_file_index_for_line.borrow() {
                                if let Some(file) = files_for_line.borrow_mut().get_mut(index) {
                                    file.additions += 1;
                                }
                            }
                        }
                        '-' => {
                            *total_deletions_clone.borrow_mut() += 1;
                            if let Some(index) = *current_file_index_for_line.borrow() {
                                if let Some(file) = files_for_line.borrow_mut().get_mut(index) {
                                    file.deletions += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                    true
                }),
            )?;
        }
        
        let files_vec = Rc::try_unwrap(files).unwrap().into_inner();
        let total_adds = *total_additions.borrow();
        let total_dels = *total_deletions.borrow();
        let files_count = files_vec.len();
        
        Ok(GitDiff {
            files: files_vec,
            stats: DiffStats {
                additions: total_adds,
                deletions: total_dels,
                files_changed: files_count,
            },
        })
    }

    pub fn stage_files(&self, paths: Vec<String>) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        let mut index = repo.index()?;
        
        for path in paths {
            let full_path = self.workspace_path.join(&path);
            
            if full_path.exists() {
                index.add_path(Path::new(&path))?;
            } else {
                // File was deleted
                index.remove_path(Path::new(&path))?;
            }
        }
        
        index.write()?;
        Ok(())
    }

    pub fn stage_all(&self) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn commit(&self, message: String) -> Result<CommitResult> {
        let repo = self.repo.lock().unwrap();
        let signature = self.get_signature_internal(&repo)?;
        let tree_id = repo.index()?.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let parent_commit = match repo.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(e) if e.code() == ErrorCode::UnbornBranch => None,
            Err(e) => return Err(e.into()),
        };
        
        let parent_commits = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
        
        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parent_commits,
        )?;
        
        let commit = repo.find_commit(oid)?;
        
        Ok(CommitResult {
            sha: oid.to_string(),
            message,
            author: signature.name().unwrap_or("Unknown").to_string(),
            timestamp: commit.time().seconds(),
        })
    }

    pub fn push(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        let mut remote = repo.find_remote(remote_name)?;
        
        // Push with authentication callback
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });
        
        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);
        
        remote.push(
            &[format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)],
            Some(&mut push_options),
        )?;
        
        Ok(())
    }

    fn get_ahead_behind_internal(&self, repo: &Repository, branch_name: &str) -> Result<(usize, usize)> {
        let local_branch = repo.find_branch(branch_name, BranchType::Local)?;
        
        if let Ok(upstream) = local_branch.upstream() {
            let local_oid = local_branch.get().target()
                .ok_or_else(|| anyhow::anyhow!("Local branch has no target"))?;
            let upstream_oid = upstream.get().target()
                .ok_or_else(|| anyhow::anyhow!("Upstream branch has no target"))?;
            
            let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;
            Ok((ahead, behind))
        } else {
            Ok((0, 0))
        }
    }

    fn get_signature_internal(&self, repo: &Repository) -> Result<Signature<'_>> {
        repo.signature()
            .or_else(|_| {
                // Fallback to default signature if git config is not set
                Signature::now("DCMT Editor", "dcmt@example.com")
            })
            .map_err(Into::into)
    }

    pub fn get_diff_as_string(&self, staged: bool) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let mut diff_options = DiffOptions::new();
        diff_options.context_lines(3);
        
        let diff = if staged {
            let head = repo.head()?.peel_to_tree()?;
            let index = repo.index()?;
            repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_options))?
        } else {
            let index = repo.index()?;
            repo.diff_index_to_workdir(Some(&index), Some(&mut diff_options))?
        };
        
        let mut output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            output.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })?;
        
        Ok(output)
    }
}

// Implement Send + Sync for GitRepository since git2::Repository is Send but not Sync
unsafe impl Send for GitRepository {}
unsafe impl Sync for GitRepository {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_git_status_serialization() {
        let status = GitStatus {
            branch: "main".to_string(),
            ahead: 2,
            behind: 1,
            staged: vec![],
            unstaged: vec![],
            untracked: vec!["new_file.txt".to_string()],
            is_initialized: true,
            has_commits: true,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: GitStatus = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.branch, "main");
        assert_eq!(deserialized.ahead, 2);
        assert_eq!(deserialized.behind, 1);
    }
}