//! Project management functionality

use crate::{FileTree, FileItem, GitFileStatus};
use std::path::{PathBuf, Path};
use serde::{Serialize, Deserialize};

#[cfg(feature = "git-integration")]
use latex_ide_git_manager::GitRepository;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub workspace_root: PathBuf,
    pub main_file: Option<PathBuf>,
    pub created: u64,
    pub modified: u64,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = get_timestamp();
        let workspace_root = path.clone();
        
        Self {
            id,
            name,
            path,
            workspace_root,
            main_file: None,
            created: now,
            modified: now,
        }
    }
    
    pub fn from_workspace(workspace_path: PathBuf) -> Self {
        let name = workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled Project")
            .to_string();
            
        let id = uuid::Uuid::new_v4().to_string();
        let now = get_timestamp();
        
        Self {
            id,
            name,
            path: workspace_path.clone(),
            workspace_root: workspace_path,
            main_file: None,
            created: now,
            modified: now,
        }
    }
    
    pub fn set_main_file(&mut self, main_file: PathBuf) {
        self.main_file = Some(main_file);
        self.modified = get_timestamp();
    }
    
    pub fn touch(&mut self) {
        self.modified = get_timestamp();
    }
}

#[derive(Clone, Debug)]
pub struct ProjectManager {
    pub current_project: Option<Project>,
    pub file_tree: Option<FileTree>,
    #[cfg(feature = "git-integration")]
    pub git_repo: Option<GitRepository>,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            current_project: None,
            file_tree: None,
            #[cfg(feature = "git-integration")]
            git_repo: None,
        }
    }
    
    pub fn open_project(&mut self, project: Project) {
        let file_tree = FileTree::new(project.path.clone());
        self.file_tree = Some(file_tree);
        
        #[cfg(feature = "git-integration")]
        {
            match GitRepository::new(project.workspace_root.clone()) {
                Ok(git_repo) => {
                    self.git_repo = Some(git_repo);
                    tracing::info!("Git repository initialized for project");
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize Git repository: {}", e);
                    self.git_repo = None;
                }
            }
        }
        
        self.current_project = Some(project);
    }
    
    pub fn close_project(&mut self) {
        self.current_project = None;
        self.file_tree = None;
        #[cfg(feature = "git-integration")]
        {
            self.git_repo = None;
        }
    }
    
    pub fn create_project(&mut self, name: String, path: PathBuf) -> Result<(), String> {
        // Create project directory if it doesn't exist
        if !path.exists() {
            return Err("Project path does not exist".to_string());
        }
        
        let project = Project::new(name, path);
        self.open_project(project);
        Ok(())
    }
    
    pub fn open_workspace(&mut self, workspace_path: PathBuf) -> Result<(), String> {
        if !workspace_path.exists() {
            return Err("Workspace path does not exist".to_string());
        }
        
        if !workspace_path.is_dir() {
            return Err("Workspace path must be a directory".to_string());
        }
        
        let mut project = Project::from_workspace(workspace_path);
        
        // Try to detect main LaTeX file
        if let Some(main_file) = self.detect_main_tex_file(&project.workspace_root) {
            project.set_main_file(main_file);
        }
        
        self.open_project(project);
        Ok(())
    }
    
    pub fn detect_main_tex_file(&self, workspace_path: &Path) -> Option<PathBuf> {
        let candidates = [
            "main.tex",
            "document.tex", 
            "thesis.tex",
            "paper.tex",
            "report.tex",
            "article.tex",
            "book.tex"
        ];
        
        // First, check common main file names
        for candidate in candidates {
            let candidate_path = workspace_path.join(candidate);
            if candidate_path.exists() {
                return Some(candidate_path.strip_prefix(workspace_path).ok()?.to_path_buf());
            }
        }
        
        // If no common names found, look for .tex files with \documentclass
        if let Ok(entries) = std::fs::read_dir(workspace_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "tex" {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if content.contains("\\documentclass") {
                                return Some(path.strip_prefix(workspace_path).ok()?.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    pub fn scan_workspace(&self) -> Result<FileTree, String> {
        if let Some(project) = &self.current_project {
            Ok(FileTree::from_directory(&project.workspace_root)?)
        } else {
            Err("No project is currently open".to_string())
        }
    }
    
    pub fn get_workspace_root(&self) -> Option<&PathBuf> {
        self.current_project.as_ref().map(|p| &p.workspace_root)
    }
    
    pub fn get_current_project(&self) -> Option<&Project> {
        self.current_project.as_ref()
    }
    
    pub fn get_file_tree(&self) -> Option<&FileTree> {
        self.file_tree.as_ref()
    }
    
    pub fn get_file_tree_mut(&mut self) -> Option<&mut FileTree> {
        self.file_tree.as_mut()
    }
    
    pub fn add_file_to_project(&mut self, path: PathBuf, name: String) -> Result<(), String> {
        if let Some(tree) = &mut self.file_tree {
            tree.add_file(path, name);
            if let Some(project) = &mut self.current_project {
                project.touch();
            }
            Ok(())
        } else {
            Err("No project is currently open".to_string())
        }
    }
    
    pub fn add_directory_to_project(&mut self, path: PathBuf, name: String) -> Result<(), String> {
        if let Some(tree) = &mut self.file_tree {
            tree.add_directory(path, name);
            if let Some(project) = &mut self.current_project {
                project.touch();
            }
            Ok(())
        } else {
            Err("No project is currently open".to_string())
        }
    }
    
    pub fn select_file(&mut self, file_id: String) {
        if let Some(tree) = &mut self.file_tree {
            tree.select_item(file_id);
        }
    }
    
    pub fn get_selected_file(&self) -> Option<&FileItem> {
        self.file_tree.as_ref()?.get_selected_item()
    }

    #[cfg(feature = "git-integration")]
    pub fn update_git_status(&mut self) -> Result<(), String> {
        // Extract git_repo to avoid borrowing self
        if let Some(ref git_repo) = self.git_repo.clone() {
            match git_repo.get_file_statuses() {
                Ok(statuses) => {
                    // Create a map of file paths to Git statuses
                    let status_map: std::collections::HashMap<String, GitFileStatus> = statuses
                        .into_iter()
                        .map(|file_status| (file_status.path, Self::convert_git_status_static(file_status.status)))
                        .collect();

                    // Update file tree with Git statuses
                    if let Some(ref mut file_tree) = self.file_tree {
                        Self::update_file_tree_git_status_static(&self.current_project, &mut file_tree.root, &status_map);
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Failed to get Git status: {}", e)),
            }
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "git-integration")]
    fn convert_git_status(&self, status: latex_ide_git_manager::GitFileStatus) -> GitFileStatus {
        Self::convert_git_status_static(status)
    }

    #[cfg(feature = "git-integration")]
    fn convert_git_status_static(status: latex_ide_git_manager::GitFileStatus) -> GitFileStatus {
        match status {
            latex_ide_git_manager::GitFileStatus::Untracked => GitFileStatus::Untracked,
            latex_ide_git_manager::GitFileStatus::Modified => GitFileStatus::Modified,
            latex_ide_git_manager::GitFileStatus::Added => GitFileStatus::Added,
            latex_ide_git_manager::GitFileStatus::Deleted => GitFileStatus::Deleted,
            latex_ide_git_manager::GitFileStatus::Renamed => GitFileStatus::Renamed,
            latex_ide_git_manager::GitFileStatus::Copied => GitFileStatus::Copied,
            latex_ide_git_manager::GitFileStatus::UpdatedButUnmerged => GitFileStatus::UpdatedButUnmerged,
            latex_ide_git_manager::GitFileStatus::Ignored => GitFileStatus::Ignored,
        }
    }

    #[cfg(feature = "git-integration")]
    fn update_file_tree_git_status(&self, item: &mut FileItem, status_map: &std::collections::HashMap<String, GitFileStatus>) {
        Self::update_file_tree_git_status_static(&self.current_project, item, status_map);
    }

    #[cfg(feature = "git-integration")]
    fn update_file_tree_git_status_static(current_project: &Option<Project>, item: &mut FileItem, status_map: &std::collections::HashMap<String, GitFileStatus>) {
        // Get the relative path of the file from workspace root
        if let Some(project) = current_project {
            if let Ok(relative_path) = item.path.strip_prefix(&project.workspace_root) {
                let path_str = relative_path.to_string_lossy().to_string();
                if let Some(git_status) = status_map.get(&path_str) {
                    item.set_git_status(Some(git_status.clone()));
                } else {
                    item.set_git_status(Some(GitFileStatus::Clean));
                }
            }
        }

        // Recursively update children
        if let Some(ref mut children) = item.children {
            for child in children.iter_mut() {
                Self::update_file_tree_git_status_static(current_project, child, status_map);
            }
        }
    }

    #[cfg(feature = "git-integration")]
    pub fn get_git_repository(&self) -> Option<&GitRepository> {
        self.git_repo.as_ref()
    }

    #[cfg(feature = "git-integration")]
    pub fn is_git_repository(&self) -> bool {
        self.git_repo.as_ref().map_or(false, |repo| repo.is_repository())
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
fn get_timestamp() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}