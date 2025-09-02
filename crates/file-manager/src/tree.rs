//! File tree management

use crate::file::FileItem;
use std::path::{PathBuf, Path};

#[derive(Clone, Debug)]
pub struct FileTree {
    pub root: FileItem,
    pub selected_item: Option<String>,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Self {
        let root_name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string();
            
        let mut root = FileItem::new_directory(root_name, root_path);
        root.set_expanded(true);
        
        Self {
            root,
            selected_item: None,
        }
    }
    
    pub fn from_directory(directory_path: &Path) -> Result<Self, String> {
        if !directory_path.exists() {
            return Err("Directory does not exist".to_string());
        }
        
        if !directory_path.is_dir() {
            return Err("Path is not a directory".to_string());
        }
        
        let root_name = directory_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string();
        
        let mut root = FileItem::new_directory(root_name, directory_path.to_path_buf());
        root.set_expanded(true);
        
        // Recursively scan directory structure
        Self::scan_directory(&mut root, directory_path)?;
        
        Ok(Self {
            root,
            selected_item: None,
        })
    }
    
    fn scan_directory(parent: &mut FileItem, dir_path: &Path) -> Result<(), String> {
        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files and directories
            if name.starts_with('.') {
                continue;
            }
            
            if path.is_dir() {
                let dir_item = FileItem::new_directory(name, path.clone());
                // Only scan first level to avoid performance issues
                // Can be expanded on demand later
                parent.add_child(dir_item);
            } else {
                let file_item = FileItem::new_file(name, path);
                parent.add_child(file_item);
            }
        }
        
        Ok(())
    }
    
    pub fn add_file(&mut self, path: PathBuf, name: String) {
        let file = FileItem::new_file(name, path);
        self.root.add_child(file);
    }
    
    pub fn add_directory(&mut self, path: PathBuf, name: String) {
        let dir = FileItem::new_directory(name, path);
        self.root.add_child(dir);
    }
    
    pub fn select_item(&mut self, item_id: String) {
        self.selected_item = Some(item_id);
    }
    
    pub fn clear_selection(&mut self) {
        self.selected_item = None;
    }
    
    pub fn get_selected_item(&self) -> Option<&FileItem> {
        let selected_id = self.selected_item.as_ref()?;
        self.find_item_by_id(&self.root, selected_id)
    }
    
    fn find_item_by_id<'a>(&self, item: &'a FileItem, id: &str) -> Option<&'a FileItem> {
        if item.id == id {
            return Some(item);
        }
        
        if let Some(children) = &item.children {
            for child in children {
                if let Some(found) = self.find_item_by_id(child, id) {
                    return Some(found);
                }
            }
        }
        
        None
    }
    
    pub fn toggle_item_expanded(&mut self, item_id: &str) {
        Self::toggle_expanded_recursive(&mut self.root, item_id);
    }
    
    fn toggle_expanded_recursive(item: &mut FileItem, id: &str) {
        if item.id == id {
            item.toggle_expanded();
            return;
        }
        
        if let Some(children) = &mut item.children {
            for child in children {
                Self::toggle_expanded_recursive(child, id);
            }
        }
    }
    
    pub fn get_all_files(&self) -> Vec<&FileItem> {
        let mut files = Vec::new();
        self.collect_files_recursive(&self.root, &mut files);
        files
    }
    
    fn collect_files_recursive<'a>(&self, item: &'a FileItem, files: &mut Vec<&'a FileItem>) {
        if item.is_file() {
            files.push(item);
        }
        
        if let Some(children) = &item.children {
            for child in children {
                self.collect_files_recursive(child, files);
            }
        }
    }
}