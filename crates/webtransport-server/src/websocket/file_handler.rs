use std::env;
use std::path::Path;
use tokio::fs;
use tracing::{info, error};

use super::{TransportMessage, FileInfo};
use crate::types::FileOp;

pub async fn handle_file_operation(operation: FileOp) -> TransportMessage {
    // Use workspace path from environment variable
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = Path::new(&workspace_path);
    
    match operation {
        FileOp::List => {
            info!("Listing files in workspace: {:?}", workspace);
            match list_workspace_files(workspace).await {
                Ok(files) => {
                    info!("Found {} files in workspace", files.len());
                    // Serialize file list as JSON and return it
                    let files_json = serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string());
                    TransportMessage::FileOperation { 
                        operation: FileOp::Download { name: files_json }
                    }
                }
                Err(e) => {
                    error!("Failed to list workspace files: {}", e);
                    TransportMessage::FileOperation { 
                        operation: FileOp::Download { name: format!("Error: {}", e) }
                    }
                }
            }
        }
        
        FileOp::Download { name } => {
            info!("Downloading file: {}", name);
            let file_path = workspace.join(&name);
            
            match fs::read(&file_path).await {
                Ok(content) => {
                    info!("Successfully read file: {} ({} bytes)", name, content.len());
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { 
                            name: "success".to_string(), 
                            content 
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { 
                            name: format!("Error: {}", e), 
                            content: vec![] 
                        }
                    }
                }
            }
        }
        
        FileOp::Upload { name, content } => {
            info!("Uploading file: {}", name);
            let file_path = workspace.join(&name);
            
            // Ensure parent directory exists
            if let Some(parent) = file_path.parent() {
                let _ = fs::create_dir_all(parent).await;
            }
            
            match fs::write(&file_path, &content).await {
                Ok(_) => {
                    info!("Successfully uploaded file: {}", name);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { name, content }
                    }
                }
                Err(e) => {
                    error!("Failed to upload file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { name: format!("Error: {}", e), content: vec![] }
                    }
                }
            }
        }
        
        FileOp::Delete { name } => {
            info!("Deleting file: {}", name);
            let file_path = workspace.join(&name);
            
            match fs::remove_file(&file_path).await {
                Ok(_) => {
                    info!("Successfully deleted file: {}", name);
                    TransportMessage::FileOperation {
                        operation: FileOp::Delete { name }
                    }
                }
                Err(e) => {
                    error!("Failed to delete file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Delete { name: format!("Error: {}", e) }
                    }
                }
            }
        }
    }
}

async fn list_workspace_files(workspace_path: &Path) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    
    if !workspace_path.exists() {
        tracing::warn!("Workspace path does not exist: {:?}", workspace_path);
        return Ok(files);
    }
    
    // Recursively scan all files in the workspace
    scan_directory_recursive(workspace_path, workspace_path, &mut files).await?;
    
    // Sort directories first, then files, maintaining hierarchy
    files.sort_by(|a, b| {
        // First sort by directory depth to maintain hierarchy
        let a_depth = a.path.matches('/').count() + a.path.matches('\\').count();
        let b_depth = b.path.matches('/').count() + b.path.matches('\\').count();
        
        match a_depth.cmp(&b_depth) {
            std::cmp::Ordering::Equal => {
                // Same depth - sort directories first, then by name
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
            other => other,
        }
    });
    
    info!("Found {} total files in workspace (including subdirectories)", files.len());
    
    Ok(files)
}

async fn scan_directory_recursive(
    current_path: &Path, 
    workspace_root: &Path, 
    files: &mut Vec<FileInfo>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut entries = fs::read_dir(current_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Skip hidden files and directories
            if name.starts_with('.') {
                continue;
            }
            
            // Create relative path from workspace root
            let relative_path = path.strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/"); // Normalize path separators
            
            files.push(FileInfo {
                name: name.to_string(),
                path: relative_path,
                is_directory: metadata.is_dir(),
                size: if metadata.is_file() { Some(metadata.len()) } else { None },
                modified: metadata.modified().ok().and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
                }),
            });
            
            // Recursively scan subdirectories
            if metadata.is_dir() {
                Box::pin(scan_directory_recursive(&path, workspace_root, files)).await?;
            }
        }
    }
    
    Ok(())
}