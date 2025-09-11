mod commands;
mod services;
mod models;
mod repo;

use commands::filesystem::*;
use commands::project::{ProjectInfo, *};
use commands::latex::*;
use commands::agent::*;
use commands::git::*;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use tokio::sync::RwLock;
use services::agent_service::AgentService;

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
            
            // Initialize agent service state - will be populated when project is selected
            let agent_service_state = Arc::new(RwLock::new(None::<AgentService>));
            app.manage(agent_service_state);
            
            // Initialize Git service state
            let git_service_state = commands::git::GitServiceState(Arc::new(std::sync::Mutex::new(None)));
            app.manage(git_service_state);
            
            info!("Application initialized - waiting for project selection");
            
            // Note: FileService, FileWatcher, and AgentService will be initialized after project selection
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
            find_main_latex_file,
            chat_with_agent,
            subscribe_to_agent_events,
            unsubscribe_from_agent_events,
            get_agent_session_info,
            list_agent_sessions,
            clear_agent_session,
            get_available_agent_tools,
            is_agent_available,
            initialize_git_service,
            get_git_status,
            get_git_diff,
            generate_commit_summary,
            stage_files,
            stage_all_files,
            commit_changes,
            push_changes,
            commit_and_push_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
