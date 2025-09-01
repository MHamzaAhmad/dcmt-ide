//! File item types and utilities

use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
    TexFile,
    BibFile,
    ImageFile,
    PdfFile,
    Unknown,
}

impl FileType {
    pub fn from_path(path: &str) -> Self {
        if path.ends_with('/') {
            return Self::Directory;
        }
        
        let extension = path.split('.').last().unwrap_or("");
        match extension.to_lowercase().as_str() {
            "tex" => Self::TexFile,
            "bib" => Self::BibFile,
            "png" | "jpg" | "jpeg" | "svg" | "gif" => Self::ImageFile,
            "pdf" => Self::PdfFile,
            "" => Self::Directory,
            _ => Self::File,
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            Self::File => "📄",
            Self::Directory => "📁",
            Self::TexFile => "📝",
            Self::BibFile => "📚",
            Self::ImageFile => "🖼️",
            Self::PdfFile => "📕",
            Self::Unknown => "❓",
        }
    }
    
    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory)
    }
    
    pub fn is_file(&self) -> bool {
        !self.is_directory()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub modified: Option<u64>,
    pub children: Option<Vec<FileItem>>,
    pub expanded: bool,
}

impl FileItem {
    pub fn new(name: String, path: PathBuf, file_type: FileType) -> Self {
        let id = generate_file_id(&path);
        let is_directory = file_type.is_directory();
        
        Self {
            id,
            name,
            path,
            file_type,
            size: None,
            modified: None,
            children: if is_directory { Some(Vec::new()) } else { None },
            expanded: false,
        }
    }
    
    pub fn new_directory(name: String, path: PathBuf) -> Self {
        Self::new(name, path, FileType::Directory)
    }
    
    pub fn new_file(name: String, path: PathBuf) -> Self {
        let file_type = FileType::from_path(&name);
        Self::new(name, path, file_type)
    }
    
    pub fn is_directory(&self) -> bool {
        self.file_type.is_directory()
    }
    
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }
    
    pub fn icon(&self) -> &'static str {
        self.file_type.icon()
    }
    
    pub fn add_child(&mut self, child: FileItem) {
        if let Some(ref mut children) = self.children {
            children.push(child);
        }
    }
    
    pub fn toggle_expanded(&mut self) {
        if self.is_directory() {
            self.expanded = !self.expanded;
        }
    }
    
    pub fn set_expanded(&mut self, expanded: bool) {
        if self.is_directory() {
            self.expanded = expanded;
        }
    }
}

fn generate_file_id(path: &PathBuf) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("file_{}", hasher.finish())
}