mod commands;
mod services;
mod models;

use commands::filesystem::*;
use commands::project::{ProjectInfo, *};
use commands::latex::*;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize project state - starts with no project selected
            let project_state = Arc::new(std::sync::RwLock::new(None::<ProjectInfo>));
            app.manage(project_state);
            
            info!("Application initialized - waiting for project selection");
            
            // Note: FileService and FileWatcher will be initialized after project selection
            // via the select_project_folder command
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_directory_tree,
            read_file_content,
            read_file_raw,
            write_file_content,
            create_file,
            delete_file,
            rename_file,
            file_exists,
            get_workspace_info,
            batch_file_operations,
            select_project_folder,
            get_current_project,
            clear_project,
            get_project_info,
            compile_latex,
            find_main_latex_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
