mod commands;
mod services;
mod models;

use commands::filesystem::*;
use services::{FileService, FileWatcher};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tracing::{info, error};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Determine workspace path - for now use current directory
            // In production, this could be user-configurable or derived from project settings
            let workspace_path = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            
            info!("Initializing file system services with workspace: {:?}", workspace_path);

            // Initialize file service
            let file_service = Arc::new(FileService::new(workspace_path.clone()));
            app.manage(file_service);

            // Initialize file watcher
            match FileWatcher::new(app_handle, workspace_path) {
                Ok(watcher) => {
                    info!("File watcher initialized successfully");
                    app.manage(watcher);
                }
                Err(e) => {
                    error!("Failed to initialize file watcher: {}", e);
                    // Continue without watcher - file operations will still work
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_directory_tree,
            read_file_content,
            write_file_content,
            create_file,
            delete_file,
            rename_file,
            file_exists,
            get_workspace_info,
            batch_file_operations
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
