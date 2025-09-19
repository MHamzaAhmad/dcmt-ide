use anyhow::{Context, Result};
use git2::{
    Branch, BranchType, Delta, DiffOptions, ErrorCode, ObjectType, Oid, Repository, RepositoryOpenFlags,
    ResetType, Signature, Status, StatusOptions, Tree,
};
use git2::build::CheckoutBuilder;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointMeta {
    pub id: String,
    pub namespace: String,
    pub title: String,
    pub created_at: i64,
    pub author: String,
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

    fn get_head_commit<'a>(repo: &'a Repository) -> Result<git2::Commit<'a>> {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit)
    }

    fn find_branch_commit<'a>(repo: &'a Repository, branch_name: &str) -> Result<git2::Commit<'a>> {
        let branch = repo.find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("Branch '{}' not found", branch_name))?;
        let commit = branch.get().peel_to_commit()?;
        Ok(commit)
    }

    pub fn ensure_checkpoint_branch(&self, namespace: &str) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let cp_branch = format!("dcmt/{}", namespace);
    if repo.find_branch(&cp_branch, git2::BranchType::Local).is_ok() {
            let commit = Self::find_branch_commit(&repo, &cp_branch)?;
            return Ok(commit.id().to_string());
        }
        // Base new branch on current HEAD
        let head_commit = Self::get_head_commit(&repo)?;
    let branch = repo.branch(&cp_branch, &head_commit, false)?;
        // Return tip id
        let tip = branch.into_reference().target().ok_or_else(|| anyhow::anyhow!("New branch has no target"))?;
        Ok(tip.to_string())
    }

    pub fn list_checkpoints(&self, namespace: &str, max: usize) -> Result<Vec<CheckpointMeta>> {
        let repo = self.repo.lock().unwrap();
        let pattern = format!("dcmt/{}/v*", namespace);
        let tag_names = repo.tag_names(Some(&pattern))?;

        // Collect (version, tag_name) pairs by parsing suffix after 'v'
        let mut versions: Vec<(u64, String)> = Vec::new();
        for name_opt in tag_names.iter().flatten() {
            if let Some(idx) = name_opt.rfind('v') {
                if let Ok(num) = name_opt[idx + 1..].parse::<u64>() {
                    versions.push((num, name_opt.to_string()));
                }
            }
        }
        // Sort descending (newest first)
        versions.sort_by(|a, b| b.0.cmp(&a.0));

        let mut out = Vec::new();
        for (i, (_v, tag_name)) in versions.into_iter().enumerate() {
            if i >= max { break; }
            let full_ref = format!("refs/tags/{}", tag_name);
            let obj = repo.revparse_single(&full_ref)?;
            // Peel to commit whether annotated or lightweight
            let commit = obj.peel_to_commit()?;
            let author = commit.author().name().unwrap_or("Unknown").to_string();
            // Title is just the short tag (e.g., v3)
            let title_line = tag_name
                .rsplit('/')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "v1".to_string());
            out.push(CheckpointMeta {
                id: commit.id().to_string(),
                namespace: namespace.to_string(),
                title: title_line,
                created_at: commit.time().seconds(),
                author,
            });
        }
        Ok(out)
    }

    pub fn diff_commits(&self, base_oid: &str, target_oid: &str) -> Result<GitDiff> {
        let repo = self.repo.lock().unwrap();
        let base = Oid::from_str(base_oid)?;
        let target = Oid::from_str(target_oid)?;
        let base_commit = repo.find_commit(base)?;
        let target_commit = repo.find_commit(target)?;
        let base_tree = base_commit.tree()?;
        let target_tree = target_commit.tree()?;

        let mut diff_options = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&target_tree), Some(&mut diff_options))?;

        let files = Rc::new(RefCell::new(Vec::new()));
        let total_additions = Rc::new(RefCell::new(0usize));
        let total_deletions = Rc::new(RefCell::new(0usize));
        let current_file_index = Rc::new(RefCell::new(None::<usize>));

        {
            let files_clone = files.clone();
            let current_file_index_clone = current_file_index.clone();
            // clones of counters not needed here; we'll use the originals below
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
                            *total_additions.borrow_mut() += 1;
                            if let Some(index) = *current_file_index_for_line.borrow() {
                                if let Some(file) = files_for_line.borrow_mut().get_mut(index) {
                                    file.additions += 1;
                                }
                            }
                        }
                        '-' => {
                            *total_deletions.borrow_mut() += 1;
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

    pub fn publish_checkpoint_fast_forward(&self, namespace: &str) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let cp_branch = format!("dcmt/{}", namespace);
        let main_branch = self.get_current_branch()?; // publish into current branch

    let cp = repo.find_branch(&cp_branch, git2::BranchType::Local)
            .with_context(|| format!("Checkpoint branch '{}' not found", cp_branch))?;
        let cp_oid = cp.get().target().ok_or_else(|| anyhow::anyhow!("Checkpoint branch has no target"))?;

        // Check if current branch is ancestor of checkpoint tip
        let head_ref = repo.find_reference(&format!("refs/heads/{}", main_branch))?;
        let head_oid = head_ref.target().ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;
    // Require fast-forward: current branch must be ancestor of checkpoint tip
    let main_is_ancestor = repo.graph_descendant_of(cp_oid, head_oid)?;
    if !main_is_ancestor {
            // allow fast-forward only when main is ancestor of checkpoint
            return Err(anyhow::anyhow!("Publish requires fast-forward: please rebase checkpoint branch onto current branch"));
        }

        // Fast-forward main to checkpoint tip
        let mut main_ref = repo.find_reference(&format!("refs/heads/{}", main_branch))?;
        main_ref.set_target(cp_oid, &format!("fast-forward {} -> {}", main_branch, cp_branch))?;

        // Update checkpoint branch to point to same commit (no-op)
        let mut cp_ref = cp.into_reference();
        cp_ref.set_target(cp_oid, &"published".to_string())?;
        Ok(cp_oid.to_string())
    }

    pub fn create_checkpoint_at_head(&self, namespace: &str, title: Option<&str>) -> Result<CheckpointMeta> {
        let repo = self.repo.lock().unwrap();
        let cp_branch = format!("dcmt/{}", namespace);

        // 1) Stage all current workdir to index to snapshot the state
        {
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
            index.write()?;
        }

        // 2) Build tree from index
        let tree_id = repo.index()?.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // 3) Determine parent (HEAD if exists)
        let parent_commit = match repo.head() {
            Ok(h) => Some(h.peel_to_commit()?),
            Err(e) if e.code() == ErrorCode::UnbornBranch => None,
            Err(e) => return Err(e.into()),
        };
        let parents: Vec<git2::Commit> = parent_commit.iter().cloned().collect();

        // 4) Create a commit object for this snapshot without moving HEAD
        let sig = self.get_signature_internal(&repo)?;
        let commit_message = title
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("checkpoint: {}", namespace));
        let new_oid = repo.commit(
            None, // do not update HEAD
            &sig,
            &sig,
            &commit_message,
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )?;
        let new_commit = repo.find_commit(new_oid)?;

        // 5) Ensure/update checkpoint branch to this new commit
        if let Ok(branch) = repo.find_branch(&cp_branch, git2::BranchType::Local) {
            let mut r = branch.into_reference();
            r.set_target(new_oid, &format!("update checkpoint {}", namespace))?;
        } else {
            let _ = repo.branch(&cp_branch, &new_commit, true)?;
        }

        let author = new_commit.author().name().unwrap_or("Unknown").to_string();
        let created_at = new_commit.time().seconds();

        // 6) Determine next version by scanning existing tags and create a new tag pointing to snapshot commit
        let pattern = format!("dcmt/{}/v*", namespace);
        let tag_names = repo.tag_names(Some(&pattern))?;
        let mut max_ver: u64 = 0;
        for name_opt in tag_names.iter().flatten() {
            if let Some(idx) = name_opt.rfind('v') {
                if let Ok(num) = name_opt[idx + 1..].parse::<u64>() {
                    if num > max_ver { max_ver = num; }
                }
            }
        }
        let next_ver = max_ver + 1;
        let tag_short = format!("v{}", next_ver);
        let tag_full = format!("dcmt/{}/{}", namespace, tag_short);
        repo.tag_lightweight(&tag_full, new_commit.as_object(), false)
            .with_context(|| format!("Failed to create checkpoint tag '{}'", tag_full))?;

        Ok(CheckpointMeta { id: new_oid.to_string(), namespace: namespace.to_string(), title: tag_short, created_at, author })
    }

    pub fn restore_checkpoint_as_commit(&self, checkpoint_oid: &str, message: &str) -> Result<CommitResult> {
        let repo = self.repo.lock().unwrap();
        // Note: we intentionally allow restoring on a dirty working directory and overwrite files.
        // This matches the product requirement: selecting a checkpoint should revert workspace files.

        let oid = Oid::from_str(checkpoint_oid)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

    // Checkout tree to working directory (force)
        let mut cb = CheckoutBuilder::new();
        cb.force();
        repo.checkout_tree(tree.as_object(), Some(&mut cb))?;

        // Update index to match tree
        let mut index = repo.index()?;
        index.read_tree(&tree)?;
        index.write()?;

    // Create a new commit on current branch with the checkpoint tree
        let signature = self.get_signature_internal(&repo)?;
        let head_commit = Self::get_head_commit(&repo)?;
        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit],
        )?;
        let new_commit = repo.find_commit(oid)?;
        Ok(CommitResult { sha: oid.to_string(), message: message.to_string(), author: signature.name().unwrap_or("Unknown").to_string(), timestamp: new_commit.time().seconds() })
    }

    pub fn repository_exists(workspace_path: &Path) -> bool {
        Repository::open_ext(workspace_path, RepositoryOpenFlags::empty(), Vec::<&Path>::new()).is_ok()
    }

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
            let files_for_line = files.clone();
            let current_file_index_for_line = current_file_index.clone();

            diff.foreach(
                &mut |delta, _progress| {
                    let path = delta
                        .new_file()
                        .path()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_string();

                    let old_path = delta
                        .old_file()
                        .path()
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
                            *total_additions.borrow_mut() += 1;
                            if let Some(index) = *current_file_index_for_line.borrow() {
                                if let Some(file) = files_for_line.borrow_mut().get_mut(index) {
                                    file.additions += 1;
                                }
                            }
                        }
                        '-' => {
                            *total_deletions.borrow_mut() += 1;
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

    pub fn get_remote_url(&self) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let remote = repo.find_remote("origin")?;
        Ok(remote.url().unwrap_or("").to_string())
    }

    pub fn get_head_oid_string(&self) -> Result<String> {
        let repo = self.repo.lock().unwrap();
        let head = repo.head()?;
        let oid = head
            .target()
            .ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;
        Ok(oid.to_string())
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

    // =============================
    // Checkpoints API
    // =============================

    fn checkpoint_branch_name(namespace: &str) -> String {
        format!("dcmt/{}", namespace)
    }

    fn ensure_checkpoint_branch_internal<'a>(repo: &'a Repository, namespace: &str) -> Result<Branch<'a>> {
        let branch_name = Self::checkpoint_branch_name(namespace);
        match repo.find_branch(&branch_name, BranchType::Local) {
            Ok(br) => Ok(br),
            Err(_) => {
                // Base on HEAD if possible
                let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
                let target_commit = head_commit
                    .ok_or_else(|| anyhow::anyhow!("Cannot create checkpoint branch without an initial commit (HEAD missing)"))?;
                let mut reference = repo.reference(
                    &format!("refs/heads/{}", branch_name),
                    target_commit.id(),
                    true,
                    &format!("create checkpoint branch {}", branch_name),
                )?;
                reference.set_target(target_commit.id(), &format!("init {} to HEAD", branch_name))?;
                let br = repo.find_branch(&branch_name, BranchType::Local)?;
                Ok(br)
            }
        }
    }

    pub fn create_checkpoint(&self, namespace: &str, title: &str, bullets: &[String]) -> Result<Oid> {
        let repo = self.repo.lock().unwrap();

        // Ensure checkpoint branch exists
        let cp_branch = Self::ensure_checkpoint_branch_internal(&repo, namespace)?;

        // Save current HEAD commit for later reset of index
        let head_commit = repo.head()?.peel_to_commit()?;

        // Stage all changes into index to capture workdir snapshot
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Parent is tip of checkpoint branch if exists, else HEAD
        let cp_target = cp_branch.get().target().unwrap_or(head_commit.id());
        let parent_commit = repo.find_commit(cp_target).or_else(|_| repo.find_commit(head_commit.id()))?;

        let sig = self.get_signature_internal(&repo)?;
        let message = serde_json::json!({
            "type": "checkpoint",
            "namespace": namespace,
            "title": title,
            "bullets": bullets,
            "createdBy": sig.name().unwrap_or("DCMT Editor"),
        }).to_string();

        let new_oid = repo.commit(
            Some(&format!("refs/heads/{}", Self::checkpoint_branch_name(namespace))),
            &sig,
            &sig,
            &message,
            &tree,
            &[&parent_commit],
        )?;

        // Reset index back to HEAD to avoid leaving everything staged
        let head_obj = repo.find_object(head_commit.id(), None)?;
        repo.reset(&head_obj, ResetType::Mixed, None)?;
        Ok(new_oid)
    }

    // removed duplicate list_checkpoints (tuple version); using the CheckpointMeta version above

    pub fn diff_between_commits(&self, base: Oid, target: Oid) -> Result<GitDiff> {
        let repo = self.repo.lock().unwrap();
        let base_tree = repo.find_commit(base)?.tree()?;
        let target_tree = repo.find_commit(target)?.tree()?;
        self.diff_trees(&repo, &base_tree, &target_tree)
    }

    fn diff_trees(&self, repo: &Repository, base: &Tree, target: &Tree) -> Result<GitDiff> {
        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(Some(base), Some(target), Some(&mut diff_opts))?;

        let files = Rc::new(RefCell::new(Vec::new()));
        let total_additions = Rc::new(RefCell::new(0usize));
        let total_deletions = Rc::new(RefCell::new(0usize));
        let current_file_index = Rc::new(RefCell::new(None::<usize>));

        {
            let files_clone = files.clone();
            let current_file_index_clone = current_file_index.clone();
            // counters used directly below
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
                            *total_additions.borrow_mut() += 1;
                            if let Some(index) = *current_file_index_for_line.borrow() {
                                if let Some(file) = files_for_line.borrow_mut().get_mut(index) {
                                    file.additions += 1;
                                }
                            }
                        }
                        '-' => {
                            *total_deletions.borrow_mut() += 1;
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

        Ok(GitDiff { files: files_vec, stats: DiffStats { additions: total_adds, deletions: total_dels, files_changed: files_count } })
    }

    pub fn restore_checkpoint(&self, namespace: &str, checkpoint_oid: Oid, commit_message: Option<String>) -> Result<CommitResult> {
        let repo = self.repo.lock().unwrap();

        // Ensure clean working directory
        let status = self.get_status()?;
        if !status.staged.is_empty() || !status.unstaged.is_empty() || !status.untracked.is_empty() {
            return Err(anyhow::anyhow!("Working directory must be clean to restore a checkpoint"));
        }

        let checkpoint_commit = repo.find_commit(checkpoint_oid)?;
        let tree = checkpoint_commit.tree()?;

        // Checkout the checkpoint tree into the working directory
        let obj = repo.find_object(tree.id(), Some(ObjectType::Tree))?;
        let mut chkb = CheckoutBuilder::new();
        chkb.force();
        repo.checkout_tree(&obj, Some(&mut chkb))?;

        // Stage all and create a new commit on current branch
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let head_commit = repo.head()?.peel_to_commit()?;
        let sig = self.get_signature_internal(&repo)?;
        let msg = commit_message.unwrap_or_else(|| format!("restore: checkpoint {} in {}", checkpoint_oid, namespace));
        let oid = repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head_commit])?;
        let commit = repo.find_commit(oid)?;

        Ok(CommitResult { sha: oid.to_string(), message: msg, author: sig.name().unwrap_or("Unknown").to_string(), timestamp: commit.time().seconds() })
    }

    pub fn publish_checkpoints_fast_forward(&self, namespace: &str, main_branch: &str) -> Result<Option<Oid>> {
        let repo = self.repo.lock().unwrap();
        // Ensure clean working directory
        let status = self.get_status()?;
        if !status.staged.is_empty() || !status.unstaged.is_empty() || !status.untracked.is_empty() {
            return Err(anyhow::anyhow!("Working directory must be clean to publish"));
        }

        let main = repo.find_branch(main_branch, BranchType::Local)?;
        let cp = Self::ensure_checkpoint_branch_internal(&repo, namespace)?;
    let _main_oid = main.get().target().ok_or_else(|| anyhow::anyhow!("main has no target"))?;
        let cp_oid = cp.get().target().ok_or_else(|| anyhow::anyhow!("checkpoint branch has no target"))?;

        // Analyze merge - only allow fast-forward for now
    let annotated = repo.find_annotated_commit(cp_oid)?;
    let (analysis, _pref) = repo.merge_analysis(&[&annotated])?;
        if analysis.is_up_to_date() {
            return Ok(None);
        }
        if analysis.is_fast_forward() {
            // FF main to cp
            let mut main_ref = repo.find_reference(&format!("refs/heads/{}", main_branch))?;
            main_ref.set_target(cp_oid, &format!("fast-forward {} to {}", main_branch, Self::checkpoint_branch_name(namespace)))?;
            // Update HEAD if currently on main
            if repo.head()?.shorthand() == Some(main_branch) {
                repo.set_head(&format!("refs/heads/{}", main_branch))?;
                let obj = repo.find_object(cp_oid, None)?;
                repo.checkout_tree(&obj, None)?;
            }
            // Fast-forward checkpoint branch back to main (no-op now that equal)
            return Ok(Some(cp_oid));
        }
        Err(anyhow::anyhow!("Non-fast-forward publish is not supported yet"))
    }
}

// Implement Send + Sync for GitRepository since git2::Repository is Send but not Sync
unsafe impl Send for GitRepository {}
unsafe impl Sync for GitRepository {}