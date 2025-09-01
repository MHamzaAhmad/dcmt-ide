//! Project management functionality

use crate::{FileTree, FileItem};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub main_file: Option<PathBuf>,
    pub created: u64,
    pub modified: u64,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = get_timestamp();
        
        Self {
            id,
            name,
            path,
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
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            current_project: None,
            file_tree: None,
        }
    }
    
    pub fn open_project(&mut self, project: Project) {
        let file_tree = FileTree::new(project.path.clone());
        self.file_tree = Some(file_tree);
        self.current_project = Some(project);
    }
    
    pub fn close_project(&mut self) {
        self.current_project = None;
        self.file_tree = None;
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