//! File system backend abstraction for cross-platform support

use crate::{FileItem, FileTree};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum FileSystemError {
    NotFound(String),
    PermissionDenied(String),
    IoError(String),
    InvalidPath(String),
}

impl std::fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "File not found: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
        }
    }
}

impl std::error::Error for FileSystemError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: u64,
    pub is_directory: bool,
    pub is_readonly: bool,
}

pub type FileSystemResult<T> = Result<T, FileSystemError>;

/// Cross-platform file system operations trait
pub trait FileSystemBackend: Send + Sync + Clone {
    /// Read directory contents
    fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<FileItem>>;
    
    /// Read file contents as string
    fn read_file(&self, path: &Path) -> FileSystemResult<String>;
    
    /// Read file contents as bytes
    fn read_file_bytes(&self, path: &Path) -> FileSystemResult<Vec<u8>>;
    
    /// Write string content to file
    fn write_file(&self, path: &Path, content: &str) -> FileSystemResult<()>;
    
    /// Write bytes to file  
    fn write_file_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()>;
    
    /// Create new file
    fn create_file(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Create directory
    fn create_dir(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Create directory and all parent directories
    fn create_dir_all(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Delete file
    fn delete_file(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Delete directory (must be empty)
    fn delete_dir(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Delete directory and all contents
    fn delete_dir_all(&self, path: &Path) -> FileSystemResult<()>;
    
    /// Rename/move file or directory
    fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()>;
    
    /// Check if path exists
    fn exists(&self, path: &Path) -> bool;
    
    /// Get file metadata
    fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata>;
    
    /// Scan directory tree
    fn scan_tree(&self, root_path: &Path) -> FileSystemResult<FileTree>;
    
    /// Watch for file system changes (optional, returns None if not supported)
    fn watch_changes(&self, _path: &Path) -> Option<Box<dyn FileSystemWatcher>> {
        None
    }
}

/// File system change events
#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// File system watcher trait
pub trait FileSystemWatcher: Send {
    /// Get next file change event (blocking)
    fn next_event(&mut self) -> Option<FileChangeEvent>;
    
    /// Try to get next event without blocking
    fn try_next_event(&mut self) -> Option<FileChangeEvent>;
    
    /// Stop watching
    fn stop(&mut self);
}

/// Native file system implementation using std::fs
#[derive(Clone)]
pub struct NativeFileSystem;

impl NativeFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemBackend for NativeFileSystem {
    fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<FileItem>> {
        let entries = std::fs::read_dir(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))?;
            
        let mut items = Vec::new();
        
        for entry in entries {
            let entry = entry.map_err(|e| FileSystemError::IoError(e.to_string()))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }
            
            let item = if path.is_dir() {
                FileItem::new_directory(name, path)
            } else {
                FileItem::new_file(name, path)
            };
            
            items.push(item);
        }
        
        // Sort directories first, then files
        items.sort_by(|a, b| {
            match (a.is_directory(), b.is_directory()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(items)
    }
    
    fn read_file(&self, path: &Path) -> FileSystemResult<String> {
        std::fs::read_to_string(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound(path.display().to_string()),
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied(path.display().to_string()),
                _ => FileSystemError::IoError(e.to_string()),
            })
    }
    
    fn read_file_bytes(&self, path: &Path) -> FileSystemResult<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound(path.display().to_string()),
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied(path.display().to_string()),
                _ => FileSystemError::IoError(e.to_string()),
            })
    }
    
    fn write_file(&self, path: &Path, content: &str) -> FileSystemResult<()> {
        std::fs::write(path, content)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied(path.display().to_string()),
                _ => FileSystemError::IoError(e.to_string()),
            })
    }
    
    fn write_file_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()> {
        std::fs::write(path, content)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied(path.display().to_string()),
                _ => FileSystemError::IoError(e.to_string()),
            })
    }
    
    fn create_file(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::File::create(path)
            .map(|_| ())
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn create_dir(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::create_dir(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn create_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::create_dir_all(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn delete_file(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::remove_file(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn delete_dir(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::remove_dir(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn delete_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        std::fs::remove_dir_all(path)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()> {
        std::fs::rename(from, to)
            .map_err(|e| FileSystemError::IoError(e.to_string()))
    }
    
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    
    fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata> {
        let meta = std::fs::metadata(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound(path.display().to_string()),
                _ => FileSystemError::IoError(e.to_string()),
            })?;
            
        let modified = meta.modified()
            .map_err(|e| FileSystemError::IoError(e.to_string()))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| FileSystemError::IoError(e.to_string()))?
            .as_millis() as u64;
            
        Ok(FileMetadata {
            size: meta.len(),
            modified,
            is_directory: meta.is_dir(),
            is_readonly: meta.permissions().readonly(),
        })
    }
    
    fn scan_tree(&self, root_path: &Path) -> FileSystemResult<FileTree> {
        FileTree::from_directory(root_path)
            .map_err(|e| FileSystemError::IoError(e))
    }
    
    #[cfg(feature = "desktop")]
    fn watch_changes(&self, path: &Path) -> Option<Box<dyn FileSystemWatcher>> {
        NativeFileWatcher::new(path).map(|w| Box::new(w) as Box<dyn FileSystemWatcher>)
    }
}

impl Default for NativeFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "desktop")]
mod native_watcher {
    use super::*;
    use notify::{RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::{channel, Receiver};
    
    pub struct NativeFileWatcher {
        _watcher: RecommendedWatcher,
        receiver: Receiver<FileChangeEvent>,
    }
    
    impl NativeFileWatcher {
        pub fn new(path: &Path) -> Option<Self> {
            let (tx, rx) = channel();
            
            let mut watcher = RecommendedWatcher::new(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let change_event = match event.kind {
                            notify::EventKind::Create(_) => FileChangeEvent::Created(path),
                            notify::EventKind::Modify(_) => FileChangeEvent::Modified(path),
                            notify::EventKind::Remove(_) => FileChangeEvent::Deleted(path),
                            _ => continue,
                        };
                        let _ = tx.send(change_event);
                    }
                }
            }, notify::Config::default()).ok()?;
            
            watcher.watch(path, RecursiveMode::Recursive).ok()?;
            
            Some(Self {
                _watcher: watcher,
                receiver: rx,
            })
        }
    }
    
    impl FileSystemWatcher for NativeFileWatcher {
        fn next_event(&mut self) -> Option<FileChangeEvent> {
            self.receiver.recv().ok()
        }
        
        fn try_next_event(&mut self) -> Option<FileChangeEvent> {
            self.receiver.try_recv().ok()
        }
        
        fn stop(&mut self) {
            // Watcher is dropped when struct is dropped
        }
    }
}

#[cfg(feature = "desktop")]
pub use native_watcher::NativeFileWatcher;