use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub modified: Option<u64>,
    pub children: Option<Vec<FileInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub content: Option<String>,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub event_type: FileEventType,
    pub path: String,
    pub metadata: FileEventMetadata,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEventMetadata {
    pub is_directory: bool,
    pub size: Option<u64>,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
}

impl FileEvent {
    pub fn new(event_type: FileEventType, path: String) -> Self {
        Self {
            event_type,
            path,
            metadata: FileEventMetadata {
                is_directory: false,
                size: None,
                old_path: None,
                new_path: None,
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn with_metadata(mut self, metadata: FileEventMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

impl FileEventMetadata {
    pub fn new(is_directory: bool) -> Self {
        Self {
            is_directory,
            size: None,
            old_path: None,
            new_path: None,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_rename(mut self, old_path: String, new_path: String) -> Self {
        self.old_path = Some(old_path);
        self.new_path = Some(new_path);
        self
    }
}

// Custom error types for file operations
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("File not found: {path}")]
    NotFound { path: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("File already exists: {path}")]
    AlreadyExists { path: String },
    
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },
    
    #[error("IO error: {source}")]
    Io { source: std::io::Error },
    
    #[error("Watch error: {message}")]
    WatchError { message: String },
}

impl From<std::io::Error> for FileError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => FileError::NotFound {
                path: "unknown".to_string(),
            },
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied {
                path: "unknown".to_string(),
            },
            std::io::ErrorKind::AlreadyExists => FileError::AlreadyExists {
                path: "unknown".to_string(),
            },
            _ => FileError::Io { source: error },
        }
    }
}

impl From<notify::Error> for FileError {
    fn from(error: notify::Error) -> Self {
        FileError::WatchError {
            message: error.to_string(),
        }
    }
}

pub type FileResult<T> = Result<T, FileError>;