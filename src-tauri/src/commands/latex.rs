use crate::commands::project::ProjectState;
use crate::models::{LaTeXCompileRequest, LaTeXCompileResponse};
use crate::services::LaTeXService;
use tauri::State;
use tracing::{debug, error, info};

#[tauri::command]
pub async fn compile_latex(
    project_state: State<'_, ProjectState>,
    request: LaTeXCompileRequest,
) -> Result<LaTeXCompileResponse, String> {
    debug!("LaTeX compilation command received with provider: {:?}", request.provider);

    // Check if a project is currently selected
    let workspace_path = {
        let state_guard = project_state.read().map_err(|e| {
            error!("Failed to acquire project state read lock: {}", e);
            "Failed to read project state".to_string()
        })?;

        match state_guard.as_ref() {
            Some(project_info) => {
                info!("Compiling LaTeX in project: {}", project_info.name);
                std::path::PathBuf::from(&project_info.path)
            }
            None => {
                return Ok(LaTeXCompileResponse::error(
                    "No project selected".to_string(),
                    vec!["Please select a project folder first".to_string()],
                ));
            }
        }
    };

    // Create LaTeX service for the current workspace
    let latex_service = LaTeXService::new(workspace_path);
    
    // Execute compilation
    match latex_service.compile_workspace(request).await {
        Ok(response) => {
            if response.success {
                info!("LaTeX compilation completed successfully");
            } else {
                error!("LaTeX compilation failed: {}", response.message);
            }
            Ok(response)
        }
        Err(e) => {
            error!("LaTeX compilation service error: {}", e);
            Ok(LaTeXCompileResponse::error(
                "Internal compilation error".to_string(),
                vec![e],
            ))
        }
    }
}

#[tauri::command]
pub async fn find_main_latex_file(
    project_state: State<'_, ProjectState>,
) -> Result<String, String> {
    debug!("Find main LaTeX file command received");

    // Check if a project is currently selected
    let workspace_path = {
        let state_guard = project_state.read().map_err(|e| {
            error!("Failed to acquire project state read lock: {}", e);
            "Failed to read project state".to_string()
        })?;

        match state_guard.as_ref() {
            Some(project_info) => {
                info!("Finding main LaTeX file in project: {}", project_info.name);
                std::path::PathBuf::from(&project_info.path)
            }
            None => {
                return Err("No project selected. Please select a project folder first".to_string());
            }
        }
    };

    // Create LaTeX service for the current workspace
    let latex_service = LaTeXService::new(workspace_path.clone());
    
    // Find main LaTeX file
    match latex_service.find_main_tex_file().await {
        Ok(tex_file_path) => {
            let relative_path = tex_file_path.strip_prefix(&workspace_path)
                .unwrap_or(&tex_file_path)
                .to_string_lossy()
                .to_string();
            
            info!("Found main LaTeX file: {}", relative_path);
            Ok(relative_path)
        }
        Err(e) => {
            error!("Failed to find main LaTeX file: {}", e);
            Err(e)
        }
    }
}