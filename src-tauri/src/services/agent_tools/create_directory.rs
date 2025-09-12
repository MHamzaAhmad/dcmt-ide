use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for creating directories in the workspace
#[derive(Clone)]
pub struct CreateDirectoryTool;

impl AgentTool for CreateDirectoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "create_directory".to_string(),
                description: "Create a new directory (and any necessary parent directories) in the workspace. Use this tool to organize your project structure by creating folders for chapters, figures, data, etc.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the directory to create within workspace (e.g., 'chapters', 'figures/diagrams')"
                        }
                    },
                    "required": ["path"]
                }),
            },
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "create_directory".to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Check if directory already exists
        if full_path.exists() {
            if full_path.is_dir() {
                return Ok(format!("Directory '{}' already exists", path));
            } else {
                return Err(AgentError::ToolExecutionError {
                    tool: "create_directory".to_string(),
                    error: format!("Path '{}' exists but is not a directory", path),
                });
            }
        }
        
        // Create directory (including parent directories)
        match fs::create_dir_all(&full_path).await {
            Ok(_) => Ok(format!("Successfully created directory '{}'", path)),
            Err(e) => Err(AgentError::ToolExecutionError {
                tool: "create_directory".to_string(),
                error: format!("Failed to create directory: {}", e),
            }),
        }
    }
}