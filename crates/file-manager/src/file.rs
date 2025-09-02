//! File item types and utilities

use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GitFileStatus {
    Untracked,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    UpdatedButUnmerged,
    Ignored,
    Clean,
}

impl GitFileStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Untracked => "❓",
            Self::Modified => "📝",
            Self::Added => "➕",
            Self::Deleted => "🗑️",
            Self::Renamed => "📝",
            Self::Copied => "📋",
            Self::UpdatedButUnmerged => "⚠️",
            Self::Ignored => "🙈",
            Self::Clean => "",
        }
    }
    
    pub fn color_class(&self) -> &'static str {
        match self {
            Self::Untracked => "text-blue-600 dark:text-blue-400",
            Self::Modified => "text-orange-600 dark:text-orange-400",
            Self::Added => "text-green-600 dark:text-green-400",
            Self::Deleted => "text-red-600 dark:text-red-400",
            Self::Renamed => "text-purple-600 dark:text-purple-400",
            Self::Copied => "text-indigo-600 dark:text-indigo-400",
            Self::UpdatedButUnmerged => "text-red-700 dark:text-red-300",
            Self::Ignored => "text-gray-400 dark:text-gray-500",
            Self::Clean => "",
        }
    }
}

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
    pub git_status: Option<GitFileStatus>,
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
            git_status: None,
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
    
    pub fn set_git_status(&mut self, status: Option<GitFileStatus>) {
        self.git_status = status;
    }
    
    pub fn git_status_icon(&self) -> &'static str {
        self.git_status.as_ref().map_or("", |status| status.icon())
    }
    
    pub fn git_status_color(&self) -> &'static str {
        self.git_status.as_ref().map_or("", |status| status.color_class())
    }
}

fn generate_file_id(path: &PathBuf) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("file_{}", hasher.finish())
}