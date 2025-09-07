use crate::services::{FileService, FileWatcher};
use crate::services::agent_service::AgentService;
use crate::services::agent_events::EventBroadcaster;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tracing::{debug, error, info, warn};
use tokio::sync::RwLock;

pub type ProjectState = Arc<std::sync::RwLock<Option<ProjectInfo>>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub selected_at: u64,
}

impl ProjectInfo {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        Self {
            name,
            path: path.to_string_lossy().to_string(),
            selected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[tauri::command]
pub async fn select_project_folder(
    app_handle: AppHandle,
    project_state: State<'_, ProjectState>,
) -> Result<Option<ProjectInfo>, String> {
    debug!("Command: select_project_folder");

    let dialog_result = app_handle
        .dialog()
        .file()
        .set_title("Select Project Folder")
        .blocking_pick_folder();

    match dialog_result {
        Some(file_path) => {
            info!("User selected project folder: {:?}", file_path);
            
            let path = file_path.into_path().map_err(|e| {
                error!("Failed to convert FilePath to PathBuf: {}", e);
                "Invalid folder path".to_string()
            })?;
            
            if !path.exists() {
                return Err("Selected folder does not exist".to_string());
            }
            
            if !path.is_dir() {
                return Err("Selected path is not a directory".to_string());
            }

            let project_info = ProjectInfo::new(path.clone());
            
            // Update project state
            {
                let mut state_guard = project_state.write().map_err(|e| {
                    error!("Failed to acquire project state write lock: {}", e);
                    "Failed to update project state".to_string()
                })?;
                *state_guard = Some(project_info.clone());
            }

            // Reinitialize services with new workspace
            if let Err(e) = reinitialize_services(&app_handle, path).await {
                error!("Failed to reinitialize services: {}", e);
                return Err(format!("Failed to initialize project: {}", e));
            }

            Ok(Some(project_info))
        }
        None => {
            debug!("User cancelled folder selection");
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn get_current_project(
    project_state: State<'_, ProjectState>,
) -> Result<Option<ProjectInfo>, String> {
    debug!("Command: get_current_project");
    
    let state_guard = project_state.read().map_err(|e| {
        error!("Failed to acquire project state read lock: {}", e);
        "Failed to read project state".to_string()
    })?;
    
    Ok(state_guard.clone())
}

#[tauri::command]
pub async fn clear_project(
    app_handle: AppHandle,
    project_state: State<'_, ProjectState>,
) -> Result<(), String> {
    debug!("Command: clear_project");
    
    // Clear project state
    {
        let mut state_guard = project_state.write().map_err(|e| {
            error!("Failed to acquire project state write lock: {}", e);
            "Failed to update project state".to_string()
        })?;
        *state_guard = None;
    }

    // Stop file services
    if let Some(_file_service) = app_handle.try_state::<Arc<FileService>>() {
        info!("Removing file service");
        // FileService doesn't need explicit cleanup, just remove from state
    }

    if let Some(file_watcher) = app_handle.try_state::<FileWatcher>() {
        info!("Stopping file watcher");
        if let Err(e) = file_watcher.stop().await {
            warn!("Failed to stop file watcher cleanly: {}", e);
        }
    }

    info!("Project cleared successfully");
    Ok(())
}

#[tauri::command]
pub async fn get_project_info(
    project_state: State<'_, ProjectState>,
) -> Result<HashMap<String, String>, String> {
    debug!("Command: get_project_info");
    
    let state_guard = project_state.read().map_err(|e| {
        error!("Failed to acquire project state read lock: {}", e);
        "Failed to read project state".to_string()
    })?;
    
    let mut info = HashMap::new();
    
    match state_guard.as_ref() {
        Some(project) => {
            info.insert("has_project".to_string(), "true".to_string());
            info.insert("project_name".to_string(), project.name.clone());
            info.insert("project_path".to_string(), project.path.clone());
            info.insert("selected_at".to_string(), project.selected_at.to_string());
        }
        None => {
            info.insert("has_project".to_string(), "false".to_string());
        }
    }
    
    Ok(info)
}

async fn reinitialize_services(app_handle: &AppHandle, workspace_path: PathBuf) -> Result<(), String> {
    info!("Reinitializing services with workspace: {:?}", workspace_path);

    // Remove existing services if they exist
    // Tauri will automatically drop the old states when we replace them

    // Initialize new file service
    let file_service = Arc::new(FileService::new(workspace_path.clone()));
    app_handle.manage(file_service);
    info!("File service reinitialized");

    // Initialize new file watcher
    match FileWatcher::new(app_handle.clone(), workspace_path.clone()) {
        Ok(watcher) => {
            info!("File watcher reinitialized successfully");
            app_handle.manage(watcher);
        }
        Err(e) => {
            error!("Failed to reinitialize file watcher: {}", e);
            // Continue without watcher - file operations will still work
        }
    }

    // Initialize agent service with default LiteLLM URL
    let litellm_url = std::env::var("LITELLM_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:4000".to_string());
    
    info!("Initializing agent service with LiteLLM URL: {}", litellm_url);
    
    let event_broadcaster = Arc::new(EventBroadcaster::new(app_handle.clone()));
    
    match AgentService::new(event_broadcaster, litellm_url).await {
        Ok(mut agent_service) => {
            agent_service.set_workspace_path(workspace_path.clone());
            
            // Update the agent service state
            if let Some(agent_state) = app_handle.try_state::<Arc<RwLock<Option<AgentService>>>>() {
                let mut agent_guard = agent_state.write().await;
                *agent_guard = Some(agent_service);
                info!("Agent service initialized successfully");
            } else {
                error!("Agent service state not found in Tauri state management");
            }
        }
        Err(e) => {
            error!("Failed to initialize agent service: {}", e);
            // Continue without agent service - other features will still work
        }
    }

    Ok(())
}