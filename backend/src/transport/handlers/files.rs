use crate::model::{CreateFileRequest, UpdateFileRequest, RenameRequest};
use crate::svc::FileService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Json as RequestJson,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

pub async fn get_directory_tree(
    Path(path): Path<String>,
    State(service): State<Arc<FileService>>,
) -> Result<Json<Value>, StatusCode> {
    let path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    info!("Getting directory tree for path: {}", path);
    
    match service.get_directory_tree(&path).await {
        Ok(tree) => Ok(Json(json!({
            "success": true,
            "data": tree
        }))),
        Err(e) => {
            error!("Failed to get directory tree: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn get_directory_tree_root(
    State(service): State<Arc<FileService>>,
) -> Result<Json<Value>, StatusCode> {
    info!("Getting root directory tree");
    
    match service.get_directory_tree("").await {
        Ok(tree) => Ok(Json(json!({
            "success": true,
            "data": tree
        }))),
        Err(e) => {
            error!("Failed to get root directory tree: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_file_content(
    Path(path): Path<String>,
    State(service): State<Arc<FileService>>,
) -> Result<Json<Value>, StatusCode> {
    let path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    info!("Getting file content for: {}", path);
    
    match service.get_file_content(&path).await {
        Ok(content) => Ok(Json(json!({
            "success": true,
            "data": content
        }))),
        Err(e) => {
            error!("Failed to get file content: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn create_file_or_directory(
    State(service): State<Arc<FileService>>,
    RequestJson(request): RequestJson<CreateFileRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Creating {} at: {}", 
          if request.is_dir { "directory" } else { "file" }, 
          request.path);
    
    match service.create_file_or_directory(request).await {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "message": "Created successfully"
        }))),
        Err(e) => {
            error!("Failed to create: {}", e);
            Err(StatusCode::CONFLICT)
        }
    }
}

pub async fn update_file_content(
    Path(path): Path<String>,
    State(service): State<Arc<FileService>>,
    RequestJson(request): RequestJson<UpdateFileRequest>,
) -> Result<Json<Value>, StatusCode> {
    let path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    info!("Updating file content for: {}", path);
    
    match service.update_file_content(&path, request).await {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "message": "File updated successfully"
        }))),
        Err(e) => {
            error!("Failed to update file: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn delete_file_or_directory(
    Path(path): Path<String>,
    State(service): State<Arc<FileService>>,
) -> Result<Json<Value>, StatusCode> {
    let path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    info!("Deleting: {}", path);
    
    match service.delete_file_or_directory(&path).await {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "message": "Deleted successfully"
        }))),
        Err(e) => {
            error!("Failed to delete: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn rename_file(
    State(service): State<Arc<FileService>>,
    RequestJson(request): RequestJson<RenameRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Renaming {} to {}", request.old_path, request.new_path);
    
    match service.rename_file(&request.old_path, &request.new_path).await {
        Ok(()) => Ok(Json(json!({
            "success": true,
            "message": "Renamed successfully"
        }))),
        Err(e) => {
            error!("Failed to rename: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn get_server_status(
    State(service): State<Arc<FileService>>,
) -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "active_connections": service.get_active_connections(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }))
}