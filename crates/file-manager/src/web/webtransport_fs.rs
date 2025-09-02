//! WebTransport-based file system implementation for web platform

use crate::fs_backend::{FileSystemBackend, FileSystemError, FileSystemResult, FileMetadata};
use crate::{FileItem, FileTree};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// Removed unused Serialize, Deserialize imports

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

// Re-export transport types for backward compatibility
pub use super::transport::{TransportMessage, FileOp};

// Transport interface is too complex for dyn compatibility
// Using concrete transport types instead

/// WebTransport-based file system for web platform
#[derive(Clone)]
pub struct WebTransportFileSystem {
    #[cfg(not(target_arch = "wasm32"))]
    _phantom: std::marker::PhantomData<()>,
    
    // Cache for file contents and metadata
    file_cache: Arc<Mutex<HashMap<PathBuf, String>>>,
    metadata_cache: Arc<Mutex<HashMap<PathBuf, FileMetadata>>>,
    file_list_cache: Arc<Mutex<Option<Vec<FileItem>>>>,
}

impl WebTransportFileSystem {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            _phantom: std::marker::PhantomData,
            
            file_cache: Arc::new(Mutex::new(HashMap::new())),
            metadata_cache: Arc::new(Mutex::new(HashMap::new())),
            file_list_cache: Arc::new(Mutex::new(None)),
        }
    }
    
    
    
    #[cfg(not(target_arch = "wasm32"))]
    async fn send_file_operation(&self, _operation: FileOp) -> FileSystemResult<TransportMessage> {
        Err(FileSystemError::IoError("WebTransport not supported on non-WASM platforms".to_string()))
    }
}

impl Default for WebTransportFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemBackend for WebTransportFileSystem {
    fn read_dir(&self, path: &Path) -> FileSystemResult<Vec<FileItem>> {
        // Check cache first
        if let Ok(cache) = self.file_list_cache.lock() {
            if let Some(files) = cache.as_ref() {
                // Filter files by path if needed
                let filtered: Vec<FileItem> = if path == Path::new("/") || path == Path::new("") {
                    files.clone()
                } else {
                    files.iter()
                        .filter(|f| f.path.parent() == Some(path))
                        .cloned()
                        .collect()
                };
                return Ok(filtered);
            }
        }
        
        // If not in cache, we need to fetch via transport
        #[cfg(target_arch = "wasm32")]
        {
            // For now, return empty list if not cached
            // In a real implementation, this would be async and fetch from server
            tracing::warn!("File list not cached, returning empty list. Call refresh_file_list() first.");
            Ok(vec![])
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        Err(FileSystemError::IoError("WebTransport not supported on non-WASM platforms".to_string()))
    }
    
    fn read_file(&self, path: &Path) -> FileSystemResult<String> {
        // Check cache first
        if let Ok(cache) = self.file_cache.lock() {
            if let Some(content) = cache.get(path) {
                return Ok(content.clone());
            }
        }
        
        // If not cached, return error for now
        // In a real implementation, this would trigger an async download
        Err(FileSystemError::NotFound(path.display().to_string()))
    }
    
    fn read_file_bytes(&self, path: &Path) -> FileSystemResult<Vec<u8>> {
        self.read_file(path).map(|s| s.into_bytes())
    }
    
    fn write_file(&self, path: &Path, content: &str) -> FileSystemResult<()> {
        // Update cache
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.insert(path.to_path_buf(), content.to_string());
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Upload file via transport
            use super::transport::upload_file;
            
            let file_name = path.display().to_string();
            let file_content = content.as_bytes().to_vec();
            
            // Spawn async task for upload
            wasm_bindgen_futures::spawn_local(async move {
                match upload_file(&file_name, file_content).await {
                    Ok(_) => tracing::info!("File uploaded successfully: {}", file_name),
                    Err(e) => tracing::error!("Failed to upload file {}: {}", file_name, e),
                }
            });
        }
        
        Ok(())
    }
    
    fn write_file_bytes(&self, path: &Path, content: &[u8]) -> FileSystemResult<()> {
        // Convert to string for now (assuming text files)
        match String::from_utf8(content.to_vec()) {
            Ok(string_content) => self.write_file(path, &string_content),
            Err(_) => {
                // For binary files, we'd need to handle differently
                Err(FileSystemError::IoError("Binary file upload not yet supported".to_string()))
            }
        }
    }
    
    fn create_file(&self, path: &Path) -> FileSystemResult<()> {
        self.write_file(path, "")
    }
    
    fn create_dir(&self, _path: &Path) -> FileSystemResult<()> {
        // Directories are implicit in our flat file structure for now
        Ok(())
    }
    
    fn create_dir_all(&self, _path: &Path) -> FileSystemResult<()> {
        // Directories are implicit in our flat file structure for now
        Ok(())
    }
    
    fn delete_file(&self, path: &Path) -> FileSystemResult<()> {
        // Remove from cache
        if let Ok(mut cache) = self.file_cache.lock() {
            cache.remove(path);
        }
        if let Ok(mut cache) = self.metadata_cache.lock() {
            cache.remove(path);
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Delete file via transport
            use super::transport::delete_file;
            
            let file_name = path.display().to_string();
            
            // Spawn async task for deletion
            wasm_bindgen_futures::spawn_local(async move {
                match delete_file(&file_name).await {
                    Ok(_) => tracing::info!("File deleted successfully: {}", file_name),
                    Err(e) => tracing::error!("Failed to delete file {}: {}", file_name, e),
                }
            });
        }
        
        Ok(())
    }
    
    fn delete_dir(&self, path: &Path) -> FileSystemResult<()> {
        self.delete_file(path)
    }
    
    fn delete_dir_all(&self, path: &Path) -> FileSystemResult<()> {
        self.delete_file(path)
    }
    
    fn rename(&self, from: &Path, to: &Path) -> FileSystemResult<()> {
        // For now, implement as copy + delete
        let content = self.read_file(from)?;
        self.write_file(to, &content)?;
        self.delete_file(from)?;
        Ok(())
    }
    
    fn exists(&self, path: &Path) -> bool {
        self.file_cache.lock()
            .map(|cache| cache.contains_key(path))
            .unwrap_or(false)
    }
    
    fn metadata(&self, path: &Path) -> FileSystemResult<FileMetadata> {
        if let Ok(cache) = self.metadata_cache.lock() {
            if let Some(metadata) = cache.get(path) {
                return Ok(metadata.clone());
            }
        }
        
        // Default metadata if not cached
        Ok(FileMetadata {
            size: self.read_file(path)?.len() as u64,
            modified: 0, // Placeholder - real implementation would get server timestamp
            is_directory: false,
            is_readonly: false,
        })
    }
    
    fn scan_tree(&self, root_path: &Path) -> FileSystemResult<FileTree> {
        let files = self.read_dir(root_path)?;
        
        let root_name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();
            
        let mut root = FileItem::new_directory(root_name, root_path.to_path_buf());
        root.set_expanded(true);
        
        for file in files {
            root.add_child(file);
        }
        
        Ok(FileTree {
            root,
            selected_item: None,
        })
    }
}

impl WebTransportFileSystem {
    /// Refresh the file list cache from the server
    #[cfg(target_arch = "wasm32")]
    pub async fn refresh_file_list(&self) -> FileSystemResult<()> {
        use super::transport::list_files;
        
        match list_files().await {
            Ok(file_infos) => {
                // Update cache with file list
                let mut cache = self.file_list_cache.lock().unwrap();
                let mut items = Vec::new();
                
                for file_info in file_infos {
                    let path = std::path::PathBuf::from(&file_info.path);
                    let item = if file_info.is_directory {
                        crate::FileItem::new_directory(file_info.name, path)
                    } else {
                        let mut item = crate::FileItem::new_file(file_info.name, path);
                        item.size = file_info.size;
                        item
                    };
                    items.push(item);
                }
                
                *cache = Some(items);
                Ok(())
            }
            Err(e) => Err(FileSystemError::IoError(format!("Failed to refresh file list: {}", e)))
        }
    }
    
    /// Download and cache a file from the server
    #[cfg(target_arch = "wasm32")]
    pub async fn download_file(&self, path: &Path) -> FileSystemResult<String> {
        use super::transport::download_file;
        
        let file_name = path.display().to_string();
        
        match download_file(&file_name).await {
            Ok(content_bytes) => {
                // Convert bytes to string and cache
                match String::from_utf8(content_bytes) {
                    Ok(content) => {
                        // Update cache
                        if let Ok(mut cache) = self.file_cache.lock() {
                            cache.insert(path.to_path_buf(), content.clone());
                        }
                        Ok(content)
                    }
                    Err(_) => Err(FileSystemError::IoError("File is not valid UTF-8".to_string()))
                }
            }
            Err(e) => Err(FileSystemError::IoError(format!("Failed to download file: {}", e)))
        }
    }
}