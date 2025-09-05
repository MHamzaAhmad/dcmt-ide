use crate::models::{FileInfo, FileContent, CreateFileRequest};
use crate::services::{FileService, emit_rename_event};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tracing::{debug, error};

pub type FileServiceState = Arc<FileService>;

#[tauri::command]
pub async fn get_directory_tree(
    path: String,
    service: State<'_, FileServiceState>,
) -> Result<FileInfo, String> {
    debug!("Command: get_directory_tree({})", path);
    
    service.get_directory_tree(&path)
        .map_err(|e| {
            error!("Failed to get directory tree for '{}': {}", path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn read_file_content(
    path: String,
    service: State<'_, FileServiceState>,
) -> Result<FileContent, String> {
    debug!("Command: read_file_content({})", path);
    
    service.read_file_content(&path)
        .map_err(|e| {
            error!("Failed to read file content for '{}': {}", path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn write_file_content(
    path: String,
    content: String,
    service: State<'_, FileServiceState>,
) -> Result<(), String> {
    debug!("Command: write_file_content({}, {} bytes)", path, content.len());
    
    service.write_file_content(&path, &content)
        .map_err(|e| {
            error!("Failed to write file content for '{}': {}", path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn create_file(
    path: String,
    content: Option<String>,
    is_directory: bool,
    service: State<'_, FileServiceState>,
) -> Result<(), String> {
    debug!("Command: create_file({}, is_dir: {})", path, is_directory);
    
    let request = CreateFileRequest {
        path,
        content,
        is_directory,
    };
    
    service.create_file_or_directory(request)
        .map_err(|e| {
            error!("Failed to create file/directory: {}", e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn delete_file(
    path: String,
    service: State<'_, FileServiceState>,
) -> Result<(), String> {
    debug!("Command: delete_file({})", path);
    
    service.delete_file_or_directory(&path)
        .map_err(|e| {
            error!("Failed to delete file/directory '{}': {}", path, e);
            e.to_string()
        })
}

#[tauri::command]
pub async fn rename_file(
    old_path: String,
    new_path: String,
    app_handle: AppHandle,
    service: State<'_, FileServiceState>,
) -> Result<(), String> {
    debug!("Command: rename_file({} -> {})", old_path, new_path);
    
    // Check if the old path is a directory before renaming
    let is_directory = service.get_workspace_path().join(&old_path).is_dir();
    
    service.rename_file(&old_path, &new_path)
        .map_err(|e| {
            error!("Failed to rename file '{}' to '{}': {}", old_path, new_path, e);
            e.to_string()
        })?;

    // Emit rename event for the frontend
    if let Err(e) = emit_rename_event(&app_handle, &old_path, &new_path, is_directory) {
        error!("Failed to emit rename event: {}", e);
        // Don't fail the operation if event emission fails
    }

    Ok(())
}

#[tauri::command]
pub async fn file_exists(
    path: String,
    service: State<'_, FileServiceState>,
) -> Result<bool, String> {
    debug!("Command: file_exists({})", path);
    
    Ok(service.file_exists(&path))
}

#[tauri::command]
pub async fn get_workspace_info(
    service: State<'_, FileServiceState>,
) -> Result<HashMap<String, String>, String> {
    debug!("Command: get_workspace_info");
    
    let mut info = HashMap::new();
    info.insert("workspace_path".to_string(), 
                service.get_workspace_path().to_string_lossy().to_string());
    
    Ok(info)
}

// Convenience command for batch operations
#[tauri::command]
pub async fn batch_file_operations(
    operations: Vec<BatchFileOperation>,
    app_handle: AppHandle,
    service: State<'_, FileServiceState>,
) -> Result<Vec<BatchResult>, String> {
    debug!("Command: batch_file_operations({} operations)", operations.len());
    
    let mut results = Vec::new();
    
    for (index, operation) in operations.into_iter().enumerate() {
        let result = match operation.operation_type.as_str() {
            "create" => {
                let request = CreateFileRequest {
                    path: operation.path.clone(),
                    content: operation.content,
                    is_directory: operation.is_directory.unwrap_or(false),
                };
                service.create_file_or_directory(request)
                    .map_err(|e| e.to_string())
            }
            "delete" => {
                service.delete_file_or_directory(&operation.path)
                    .map_err(|e| e.to_string())
            }
            "rename" => {
                if let Some(new_path) = operation.new_path {
                    let is_directory = service.get_workspace_path().join(&operation.path).is_dir();
                    service.rename_file(&operation.path, &new_path)
                        .map_err(|e| e.to_string())?;
                    
                    // Emit rename event
                    if let Err(e) = emit_rename_event(&app_handle, &operation.path, &new_path, is_directory) {
                        error!("Failed to emit rename event: {}", e);
                    }
                    Ok(())
                } else {
                    Err("New path required for rename operation".to_string())
                }
            }
            _ => Err(format!("Unknown operation type: {}", operation.operation_type))
        };

        let success = result.is_ok();
        let error = result.as_ref().err().map(|e| e.clone());
        
        results.push(BatchResult {
            index,
            success,
            error,
        });

        // Stop on first error if requested
        if let Err(_) = result {
            if operation.stop_on_error.unwrap_or(false) {
                break;
            }
        }
    }
    
    Ok(results)
}

#[derive(serde::Deserialize)]
pub struct BatchFileOperation {
    pub operation_type: String, // "create", "delete", "rename"
    pub path: String,
    pub new_path: Option<String>, // for rename operations
    pub content: Option<String>, // for create operations
    pub is_directory: Option<bool>, // for create operations
    pub stop_on_error: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct BatchResult {
    pub index: usize,
    pub success: bool,
    pub error: Option<String>,
}