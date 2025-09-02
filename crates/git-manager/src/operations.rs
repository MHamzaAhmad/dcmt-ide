use crate::GitRepository;
use anyhow::{anyhow, Result};
use git2::{BranchType, ObjectType, Oid, ResetType, Signature};
use std::path::Path;
use tracing::{debug, info};

pub struct GitOperations {
    git_repo: GitRepository,
}

impl GitOperations {
    pub fn new(git_repo: GitRepository) -> Self {
        Self { git_repo }
    }

    pub fn stage_file(&self, file_path: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let mut index = repo.index()?;
        index.add_path(Path::new(file_path))?;
        index.write()?;

        debug!("Staged file: {}", file_path);
        Ok(())
    }

    pub fn unstage_file(&self, file_path: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let mut index = repo.index()?;
        index.remove_path(Path::new(file_path))?;
        index.write()?;

        debug!("Unstaged file: {}", file_path);
        Ok(())
    }

    pub fn stage_all_changes(&self) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        info!("Staged all changes");
        Ok(())
    }

    pub fn discard_file_changes(&self, file_path: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Get HEAD commit
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let head_tree = head_commit.tree()?;

        // Checkout the file from HEAD
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.path(file_path);
        checkout_builder.force();
        repo.checkout_tree(head_tree.as_object(), Some(&mut checkout_builder))?;

        info!("Discarded changes for file: {}", file_path);
        Ok(())
    }

    pub fn discard_all_changes(&self) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Reset to HEAD, discarding all changes
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        
        repo.reset(&head_commit.into_object(), ResetType::Hard, None)?;

        info!("Discarded all changes");
        Ok(())
    }

    pub fn create_branch(&self, branch_name: &str, from_current: bool) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let commit = if from_current {
            repo.head()?.peel_to_commit()?
        } else {
            // Create from base branch
            let base_branch_name = self.git_repo.get_base_branch();
            let base_branch = repo.find_branch(base_branch_name, BranchType::Local)?;
            base_branch.get().peel_to_commit()?
        };

        let _branch = repo.branch(branch_name, &commit, false)?;
        info!("Created branch: {}", branch_name);
        Ok(())
    }

    pub fn delete_branch(&self, branch_name: &str, force: bool) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Don't allow deleting current branch
        if let Ok(head) = repo.head() {
            if let Some(current_branch) = head.shorthand() {
                if current_branch == branch_name {
                    return Err(anyhow!("Cannot delete current branch: {}", branch_name));
                }
            }
        }

        let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
        
        if !force {
            // Check if branch is merged
            // For now, we'll skip this check and implement it later if needed
        }

        branch.delete()?;
        info!("Deleted branch: {}", branch_name);
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        let repo = self.git_repo.get_repository()?;

        let branches = repo.branches(Some(BranchType::Local))?;
        let mut branch_names = Vec::new();

        for branch_result in branches {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                branch_names.push(name.to_string());
            }
        }

        Ok(branch_names)
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let repo = self.git_repo.get_repository()?;

        let head = repo.head()?;
        if head.is_branch() {
            Ok(head.shorthand().unwrap_or("HEAD").to_string())
        } else {
            Ok("HEAD".to_string())
        }
    }

    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let branch_ref = branch.get();
        
        repo.checkout_tree(
            &branch_ref.peel(ObjectType::Tree)?,
            Some(git2::build::CheckoutBuilder::new().safe()),
        )?;
        repo.set_head(branch_ref.name().unwrap())?;

        info!("Checked out branch: {}", branch_name);
        Ok(())
    }

    pub fn merge_branch(&self, branch_name: &str, message: Option<&str>) -> Result<Oid> {
        let repo = self.git_repo.get_repository()?;

        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let branch_commit = branch.get().peel_to_commit()?;
        let current_commit = repo.head()?.peel_to_commit()?;

        // Create annotated commit for merge analysis
        let annotated_commit = repo.find_annotated_commit(branch_commit.id())?;
        let analysis = repo.merge_analysis(&[&annotated_commit])?;

        if analysis.0.is_up_to_date() {
            info!("Already up to date with branch: {}", branch_name);
            return Ok(current_commit.id());
        } else if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let mut reference = repo.head()?;
            reference.set_target(branch_commit.id(), "Fast-forward merge")?;
            repo.set_head(reference.name().unwrap())?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            
            info!("Fast-forward merge of branch: {}", branch_name);
            return Ok(branch_commit.id());
        } else if analysis.0.is_normal() {
            // Normal merge
            let mut index = repo.merge_commits(&current_commit, &branch_commit, None)?;
            
            if index.has_conflicts() {
                return Err(anyhow!("Merge conflicts detected with branch: {}", branch_name));
            }

            let tree_id = index.write_tree_to(&repo)?;
            let tree = repo.find_tree(tree_id)?;
            let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;
            
            let default_message = format!("Merge branch '{}'", branch_name);
            let merge_message = message.unwrap_or(&default_message);
            
            let commit_id = repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                merge_message,
                &tree,
                &[&current_commit, &branch_commit],
            )?;

            repo.cleanup_state()?;
            info!("Merged branch '{}' with commit: {}", branch_name, commit_id);
            return Ok(commit_id);
        }

        Err(anyhow!("Cannot merge branch: {}", branch_name))
    }

    pub fn revert_commit(&self, commit_id: &str) -> Result<Oid> {
        let repo = self.git_repo.get_repository()?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        
        // Create revert commit
        let mut revert_index = repo.revert_commit(&commit, &repo.head()?.peel_to_commit()?, 0, None)?;
        
        if revert_index.has_conflicts() {
            return Err(anyhow!("Conflicts detected while reverting commit: {}", commit_id));
        }

        let tree_id = revert_index.write_tree_to(&repo)?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;
        let parent_commit = repo.head()?.peel_to_commit()?;
        
        let revert_message = format!("Revert commit {}", commit_id);
        
        let revert_commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &revert_message,
            &tree,
            &[&parent_commit],
        )?;

        info!("Reverted commit {} with new commit: {}", commit_id, revert_commit_id);
        Ok(revert_commit_id)
    }

    pub fn reset_to_commit(&self, commit_id: &str, reset_type: ResetType) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        
        repo.reset(&commit.into_object(), reset_type, None)?;
        
        info!("Reset to commit: {} with type: {:?}", commit_id, reset_type);
        Ok(())
    }

    pub fn push_to_remote(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let mut remote = repo.find_remote(remote_name)?;
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        
        // For now, we'll implement a basic push without authentication
        // In a real implementation, you'd want to handle authentication properly
        remote.push(&[&refspec], None)?;
        
        info!("Pushed branch '{}' to remote '{}'", branch_name, remote_name);
        Ok(())
    }

    pub fn pull_from_remote(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        // Fetch from remote
        let mut remote = repo.find_remote(remote_name)?;
        remote.fetch(&[branch_name], None, None)?;
        
        // Get the fetched commit
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = fetch_head.peel_to_commit()?;
        
        // Create annotated commit for merge analysis
        let annotated_commit = repo.find_annotated_commit(fetch_commit.id())?;
        let analysis = repo.merge_analysis(&[&annotated_commit])?;
        
        if analysis.0.is_up_to_date() {
            info!("Already up to date");
        } else if analysis.0.is_fast_forward() {
            let mut reference = repo.head()?;
            reference.set_target(fetch_commit.id(), "Fast-forward pull")?;
            repo.set_head(reference.name().unwrap())?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            info!("Fast-forward pull completed");
        } else {
            return Err(anyhow!("Pull requires merge - not implemented yet"));
        }
        
        Ok(())
    }

    pub fn get_git_repository(&self) -> &GitRepository {
        &self.git_repo
    }
}