use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
    pub children: Option<Vec<FileInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub is_dir: bool,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub operation: String,
    pub path: String,
    pub success: bool,
    pub message: Option<String>,
}

impl FileInfo {
    pub fn new(
        path: String,
        name: String,
        is_dir: bool,
        size: Option<u64>,
        modified: Option<u64>,
    ) -> Self {
        Self {
            path,
            name,
            is_dir,
            size,
            modified,
            children: if is_dir { Some(Vec::new()) } else { None },
        }
    }

    pub fn add_child(&mut self, child: FileInfo) {
        if let Some(ref mut children) = self.children {
            children.push(child);
        }
    }
}

pub fn sanitize_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    
    // Check for path traversal attempts
    if path.components().any(|comp| match comp {
        std::path::Component::ParentDir => true,
        _ => false,
    }) {
        return Err("Path traversal detected".to_string());
    }

    Ok(path)
}