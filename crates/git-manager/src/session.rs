use crate::{GitRepository, GitStatus};
use anyhow::Result;
use chrono::Utc;
use git2::{BranchType, Oid, Repository, Signature};
use tracing::{info, warn};
use std::fs;

pub struct SessionManager {
    git_repo: GitRepository,
}

impl SessionManager {
    pub fn new(git_repo: GitRepository) -> Self {
        Self { git_repo }
    }

    pub fn start_session(&mut self) -> Result<String> {
        // Initialize repository if it doesn't exist
        if !self.git_repo.is_repository() {
            info!("Repository not found, initializing new repository");
            self.git_repo.init_repository()?;
        }

        // Check if we're already in a session
        if let Some(existing_session) = &self.git_repo.session_branch {
            warn!("Already in session: {}", existing_session);
            return Ok(existing_session.clone());
        }

        let repo = self.git_repo.get_repository()?;

        // Check if repository has any commits
        if repo.is_empty()? {
            info!("Repository is empty, creating initial commit");
            self.create_initial_commit(&repo)?;
        }

        // Get base branch name before borrowing repository
        let base_branch_name = self.git_repo.get_base_branch().to_string();

        // Generate unique session branch name with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let mut session_branch_name = format!("session-{}", timestamp);
        let mut counter = 1;

        // Ensure session branch name is unique
        while repo.find_branch(&session_branch_name, BranchType::Local).is_ok() {
            session_branch_name = format!("session-{}-{}", timestamp, counter);
            counter += 1;
            if counter > 1000 {
                return Err(anyhow::anyhow!("Unable to create unique session branch name"));
            }
        }

        // Ensure we're on the base branch before creating session branch
        match self.ensure_base_branch(&repo) {
            Ok(_) => {},
            Err(e) => {
                warn!("Failed to ensure base branch, continuing anyway: {}", e);
            }
        }

        // Create session branch from base branch
        let base_branch = repo.find_branch(&base_branch_name, BranchType::Local)
            .map_err(|e| anyhow::anyhow!("Base branch '{}' not found: {}", base_branch_name, e))?;
        let base_commit = base_branch.get().peel_to_commit()
            .map_err(|e| anyhow::anyhow!("Failed to get base branch commit: {}", e))?;

        // Create new session branch
        let mut session_branch = repo.branch(&session_branch_name, &base_commit, false)
            .map_err(|e| anyhow::anyhow!("Failed to create session branch '{}': {}", session_branch_name, e))?;
        
        // Checkout the session branch
        let session_ref = session_branch.get();
        let tree_object = session_ref.peel(git2::ObjectType::Tree)?;
        match repo.checkout_tree(
            &tree_object,
            Some(git2::build::CheckoutBuilder::new().force()),
        ) {
            Ok(_) => {
                repo.set_head(session_ref.name().unwrap())?;
                self.git_repo.session_branch = Some(session_branch_name.clone());
                
                info!("Started new session: {}", session_branch_name);
                Ok(session_branch_name)
            }
            Err(e) => {
                // Clean up the branch we just created
                let _ = session_branch.delete();
                Err(anyhow::anyhow!("Failed to checkout session branch: {}", e))
            }
        }
    }

    pub fn end_session(&mut self, save_changes: bool) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        if let Some(session_branch_name) = &self.git_repo.session_branch {
            if save_changes {
                // Commit any pending changes before ending session
                self.commit_session_changes("End of session - auto commit")?;
            }

            // Switch back to base branch
            let base_branch_name = self.git_repo.get_base_branch();
            self.checkout_branch(base_branch_name)?;

            if !save_changes {
                // Delete the session branch if not saving changes
                let mut session_branch = repo.find_branch(session_branch_name, BranchType::Local)?;
                session_branch.delete()?;
                info!("Deleted session branch: {}", session_branch_name);
            } else {
                info!("Session branch preserved: {}", session_branch_name);
            }

            self.git_repo.session_branch = None;
        }

        Ok(())
    }

    pub fn commit_session_changes(&self, message: &str) -> Result<Oid> {
        let repo = self.git_repo.get_repository()?;

        // Add all changes to staging
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Create commit
        let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let parent_commit = if let Ok(head) = repo.head() {
            Some(head.peel_to_commit()?)
        } else {
            None
        };

        let parents: Vec<&git2::Commit> = if let Some(ref commit) = parent_commit {
            vec![commit]
        } else {
            vec![]
        };

        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        info!("Created commit: {} - {}", commit_id, message);
        Ok(commit_id)
    }

    pub fn commit_pdf_version(&self, pdf_path: &str, _latex_content: &str) -> Result<(Oid, String)> {
        let version = self.get_next_version()?;
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let message = format!(
            "PDF compilation successful - {} ({})\n\nPDF: {}\nCompiled with LaTeX IDE",
            timestamp, version, pdf_path
        );

        let commit_id = self.commit_session_changes(&message)?;
        self.increment_version_file(&version)?;
        
        // Create version tag
        self.create_version_tag(&version, &commit_id)?;
        
        info!("Created versioned commit: {} - {}", commit_id, version);
        Ok((commit_id, version))
    }

    pub fn save_session_with_tag(&self, tag_name: &str, message: &str) -> Result<(Oid, String)> {
        let repo = self.git_repo.get_repository()?;

        // Commit current changes
        let commit_message = format!("Save session: {}", message);
        let commit_id = self.commit_session_changes(&commit_message)?;

        // Create tag
        let commit = repo.find_commit(commit_id)?;
        let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;
        let tag_message = format!("Session saved: {}\n{}", tag_name, message);
        
        let tag_id = repo.tag(
            tag_name,
            &commit.into_object(),
            &signature,
            &tag_message,
            false,
        )?;

        info!("Created tag: {} for commit: {}", tag_name, commit_id);
        Ok((commit_id, tag_id.to_string()))
    }

    pub fn get_session_status(&self) -> Result<GitStatus> {
        self.git_repo.get_status()
    }

    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let repo = self.git_repo.get_repository()?;

        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let branch_ref = branch.get();
        
        repo.checkout_tree(
            &branch_ref.peel(git2::ObjectType::Tree)?,
            Some(git2::build::CheckoutBuilder::new().safe()),
        )?;
        repo.set_head(branch_ref.name().unwrap())?;

        info!("Checked out branch: {}", branch_name);
        Ok(())
    }

    fn ensure_base_branch(&self, repo: &Repository) -> Result<()> {
        let base_branch_name = self.git_repo.get_base_branch();
        
        // Check if base branch exists
        match repo.find_branch(base_branch_name, BranchType::Local) {
            Ok(_) => {
                // Branch exists, check it out
                self.checkout_branch(base_branch_name)?;
                Ok(())
            }
            Err(_) => {
                // Base branch doesn't exist, create it
                warn!("Base branch '{}' not found, creating it", base_branch_name);
                
                // Create initial commit if repository is empty
                if repo.is_empty()? {
                    self.create_initial_commit(repo)?;
                }

                // Create the base branch
                let head_commit = repo.head()?.peel_to_commit()?;
                let _base_branch = repo.branch(base_branch_name, &head_commit, false)?;
                self.checkout_branch(base_branch_name)?;
                
                Ok(())
            }
        }
    }

    fn create_initial_commit(&self, repo: &Repository) -> Result<Oid> {
        // Create initial .gitignore for LaTeX projects
        let gitignore_content = r#"# LaTeX auxiliary files
*.aux
*.lof
*.log
*.lot
*.fls
*.out
*.toc
*.fmt
*.fot
*.cb
*.cb2
*.fdb_latexmk
*.fls
*.figlist
*.makefile
*.fgn
*.fgw
*.figlist
*.makefile

# LaTeX intermediate files
*.dvi
*.xdv
*-converted-to.*

# BibTeX auxiliary files
*.bbl
*.bcf
*.blg
*.run.xml

# Build directories
build/
out/
dist/

# IDE files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
"#;

        std::fs::write(self.git_repo.get_repo_path().join(".gitignore"), gitignore_content)?;

        // Add and commit
        let mut index = repo.index()?;
        index.add_path(std::path::Path::new(".gitignore"))?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;

        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit - LaTeX IDE project",
            &tree,
            &[],
        )?;

        info!("Created initial commit: {}", commit_id);
        Ok(commit_id)
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let status = self.get_session_status()?;
        Ok(status.has_changes)
    }

    pub fn get_git_repository(&self) -> &GitRepository {
        &self.git_repo
    }

    pub fn get_git_repository_mut(&mut self) -> &mut GitRepository {
        &mut self.git_repo
    }

    fn get_version_file_path(&self) -> std::path::PathBuf {
        self.git_repo.get_repo_path().join(".latex_version")
    }

    fn get_next_version(&self) -> Result<String> {
        let version_file = self.get_version_file_path();
        
        if version_file.exists() {
            let content = match fs::read_to_string(&version_file) {
                Ok(content) => content,
                Err(e) => {
                    warn!("Failed to read version file, using default version: {}", e);
                    return Ok("v1.0.1".to_string());
                }
            };
            
            let current = content.trim();
            
            // Parse version (assumes format like "v1.0.123") 
            if let Ok(regex) = regex::Regex::new(r"^v(\d+)\.(\d+)\.(\d+)$") {
                if let Some(captures) = regex.captures(current) {
                    let major: u32 = captures[1].parse().unwrap_or(1);
                    let minor: u32 = captures[2].parse().unwrap_or(0);
                    let patch: u32 = captures[3].parse().unwrap_or(0);
                    
                    // Handle potential overflow
                    if patch == u32::MAX {
                        warn!("Patch version at maximum, incrementing minor version");
                        Ok(format!("v{}.{}.{}", major, minor + 1, 0))
                    } else {
                        Ok(format!("v{}.{}.{}", major, minor, patch + 1))
                    }
                } else {
                    warn!("Invalid version format in file: {}, using default", current);
                    Ok("v1.0.1".to_string())
                }
            } else {
                warn!("Failed to compile version regex, using default");
                Ok("v1.0.1".to_string())
            }
        } else {
            // First version
            Ok("v1.0.1".to_string())
        }
    }

    fn increment_version_file(&self, version: &str) -> Result<()> {
        let version_file = self.get_version_file_path();
        fs::write(version_file, version)?;
        Ok(())
    }

    fn create_version_tag(&self, version: &str, commit_id: &Oid) -> Result<()> {
        let repo = self.git_repo.get_repository()?;
        let commit = repo.find_commit(*commit_id)?;
        let signature = Signature::now("LaTeX IDE", "latex-ide@localhost")?;
        let tag_message = format!("PDF version {}", version);
        
        repo.tag(
            version,
            &commit.into_object(),
            &signature,
            &tag_message,
            false,
        )?;

        info!("Created version tag: {} for commit: {}", version, commit_id);
        Ok(())
    }

    pub fn get_current_version(&self) -> Result<Option<String>> {
        let version_file = self.get_version_file_path();
        
        if version_file.exists() {
            let content = fs::read_to_string(&version_file)?;
            Ok(Some(content.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_versions(&self) -> Result<Vec<String>> {
        let repo = self.git_repo.get_repository()?;
        let mut versions = Vec::new();

        // Get all tags that match version pattern
        repo.tag_foreach(|_oid, name| {
            if let Ok(name_str) = std::str::from_utf8(name) {
                if let Some(tag_name) = name_str.strip_prefix("refs/tags/") {
                    if tag_name.starts_with("v") && tag_name.matches('.').count() == 2 {
                        versions.push(tag_name.to_string());
                    }
                }
            }
            true
        })?;

        // Sort versions (simple string sort works for semantic versions)
        versions.sort();
        versions.reverse(); // Most recent first
        
        Ok(versions)
    }

    pub fn rollback_to_version(&mut self, version: &str) -> Result<()> {
        // Validate version format
        if !version.starts_with("v") || !version.contains('.') {
            return Err(anyhow::anyhow!("Invalid version format: {}", version));
        }
        
        let repo = self.git_repo.get_repository()?;
        
        // Check if we have uncommitted changes
        if self.has_uncommitted_changes()? {
            warn!("Uncommitted changes detected, they will be lost during rollback");
        }
        
        // Find the tag
        let tag_ref_name = format!("refs/tags/{}", version);
        let tag_ref = repo.find_reference(&tag_ref_name)
            .map_err(|_| anyhow::anyhow!("Version tag '{}' not found", version))?;
        
        let tag_commit = tag_ref.peel_to_commit()
            .map_err(|e| anyhow::anyhow!("Failed to get commit for version {}: {}", version, e))?;
        
        // Backup current state if needed
        let current_head = repo.head().ok();
        
        // Checkout the commit
        match repo.checkout_tree(
            tag_commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        ) {
            Ok(_) => {
                // Update HEAD
                repo.set_head_detached(tag_commit.id())?;
                
                // Update version file to match rollback
                self.increment_version_file(version)?;
                
                info!("Successfully rolled back to version: {}", version);
                Ok(())
            }
            Err(e) => {
                warn!("Rollback failed, attempting to restore previous state: {}", e);
                
                // Try to restore previous state
                if let Some(head) = current_head {
                    if let Ok(commit) = head.peel_to_commit() {
                        let _ = repo.checkout_tree(commit.as_object(), None);
                        let _ = repo.set_head(head.name().unwrap_or("HEAD"));
                    }
                }
                
                Err(anyhow::anyhow!("Rollback to version {} failed: {}", version, e))
            }
        }
    }

    pub fn rollback_to_previous_commit(&mut self) -> Result<String> {
        let repo = self.git_repo.get_repository()?;
        
        // Get current HEAD
        let head = repo.head()
            .map_err(|_| anyhow::anyhow!("No commits found in repository"))?;
        let current_commit = head.peel_to_commit()?;
        
        // Check if there are parent commits
        if current_commit.parent_count() == 0 {
            return Err(anyhow::anyhow!("Cannot rollback: this is the initial commit"));
        }
        
        // Get parent commit
        let parent_commit = current_commit.parent(0)?;
        
        // Check if we have uncommitted changes
        if self.has_uncommitted_changes()? {
            warn!("Uncommitted changes detected, they will be lost during rollback");
        }
        
        // Perform rollback
        repo.checkout_tree(
            parent_commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )?;
        
        // Update HEAD
        repo.set_head_detached(parent_commit.id())?;
        
        let rolled_back_id = parent_commit.id().to_string();
        info!("Rolled back to previous commit: {}", rolled_back_id);
        
        Ok(rolled_back_id)
    }
}