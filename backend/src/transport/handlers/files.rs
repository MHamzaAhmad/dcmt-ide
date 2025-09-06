use crate::model::{CreateFileRequest, UpdateFileRequest, RenameRequest};
use crate::svc::FileService;
use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Json, IntoResponse},
    Json as RequestJson,
    body::Bytes,
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

pub async fn get_file_raw(
    Path(path): Path<String>,
    State(service): State<Arc<FileService>>,
) -> Result<impl IntoResponse, StatusCode> {
    let path = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?.to_string();
    
    info!("Getting raw file content for: {}", path);
    
    match service.get_file_raw(&path).await {
        Ok(content) => {
            // Determine content type based on file extension
            let content_type = get_content_type(&path);
            
            Ok((
                [(header::CONTENT_TYPE, content_type)],
                Bytes::from(content)
            ))
        }
        Err(e) => {
            error!("Failed to get raw file content: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

fn get_content_type(path: &str) -> &'static str {
    let ext = path.split('.').next_back().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" | "tex" | "log" | "aux" => "text/plain",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}