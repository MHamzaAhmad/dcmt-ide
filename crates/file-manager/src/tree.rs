//! File tree management

use crate::file::FileItem;
use std::path::PathBuf;

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