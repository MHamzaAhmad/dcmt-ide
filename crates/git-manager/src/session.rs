use crate::{GitRepository, GitStatus};
use anyhow::Result;
use chrono::Utc;
use git2::{BranchType, Oid, Repository, Signature};
use tracing::{info, warn};

pub struct SessionManager {
    git_repo: GitRepository,
}

impl SessionManager {
    pub fn new(git_repo: GitRepository) -> Self {
        Self { git_repo }
    }

    pub fn start_session(&mut self) -> Result<String> {
        if !self.git_repo.is_repository() {
            self.git_repo.init_repository()?;
        }

        // Get base branch name before borrowing repository
        let base_branch_name = self.git_repo.get_base_branch().to_string();

        let repo = self.git_repo.get_repository()?;

        // Generate session branch name with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let session_branch_name = format!("session-{}", timestamp);

        // Ensure we're on the base branch before creating session branch
        self.ensure_base_branch(&repo)?;

        // Create session branch from base branch
        let base_branch = repo.find_branch(&base_branch_name, BranchType::Local)?;
        let base_commit = base_branch.get().peel_to_commit()?;

        // Create new session branch
        let session_branch = repo.branch(&session_branch_name, &base_commit, false)?;
        
        // Checkout the session branch
        let session_ref = session_branch.get();
        repo.checkout_tree(
            &session_ref.peel(git2::ObjectType::Tree)?,
            Some(git2::build::CheckoutBuilder::new().force()),
        )?;
        repo.set_head(session_ref.name().unwrap())?;

        self.git_repo.session_branch = Some(session_branch_name.clone());
        
        info!("Started new session: {}", session_branch_name);
        Ok(session_branch_name)
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

    pub fn commit_pdf_version(&self, pdf_path: &str, _latex_content: &str) -> Result<Oid> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let message = format!(
            "PDF compilation successful - {}\n\nPDF: {}\nCompiled with LaTeX IDE",
            timestamp, pdf_path
        );

        self.commit_session_changes(&message)
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
}