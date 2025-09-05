use crate::svc::FileService;
use crate::transport::handlers::files;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

pub fn files_router() -> Router<Arc<FileService>> {
    Router::new()
        // File tree endpoints
        .route("/tree", get(files::get_directory_tree_root))
        .route("/tree/*path", get(files::get_directory_tree))
        
        // File content endpoints
        .route("/content/*path", get(files::get_file_content))
        
        // File operations
        .route("/", post(files::create_file_or_directory))
        .route("/*path", put(files::update_file_content))
        .route("/*path", delete(files::delete_file_or_directory))
        
        // Rename endpoint
        .route("/rename/*old_path/*new_path", post(files::rename_file))
        
        // Server status
        .route("/status", get(files::get_server_status))
}