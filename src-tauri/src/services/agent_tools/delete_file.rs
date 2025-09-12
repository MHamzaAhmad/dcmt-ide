use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for deleting files from the workspace
#[derive(Clone)]
pub struct DeleteFileTool;

impl AgentTool for DeleteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "delete_file".to_string(),
                description: "Delete a file from the workspace. This is a destructive operation that cannot be undone. Use with caution and only when you're certain the file should be removed.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace (e.g., 'old_file.txt', 'temp/draft.tex')"
                        },
                        "confirm": {
                            "type": "boolean",
                            "description": "Must be set to true to confirm the deletion (safety measure)"
                        }
                    },
                    "required": ["path", "confirm"]
                }),
                display_name: Some("Delete File".to_string()),
                progressive_form: Some("Deleting file".to_string()),
            },
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "delete_file".to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
            
        let confirm = args["confirm"]
            .as_bool()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "delete_file".to_string(), 
                error: "Missing 'confirm' parameter".to_string() 
            })?;
        
        // Safety check - require explicit confirmation
        if !confirm {
            return Err(AgentError::InvalidToolArguments {
                tool: "delete_file".to_string(),
                error: "Confirmation required: set 'confirm' to true to delete the file".to_string(),
            });
        }
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Check if file exists
        if !full_path.exists() {
            return Err(AgentError::ToolExecutionError {
                tool: "delete_file".to_string(),
                error: format!("File not found: {}", path),
            });
        }
        
        // Check if it's actually a file (not a directory)
        if !full_path.is_file() {
            return Err(AgentError::ToolExecutionError {
                tool: "delete_file".to_string(),
                error: format!("Path '{}' is not a file (use a directory removal tool for directories)", path),
            });
        }
        
        // Get file metadata before deletion for confirmation message
        let metadata = fs::metadata(&full_path).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: "delete_file".to_string(),
                error: format!("Failed to read file metadata: {}", e),
            })?;
        
        let file_size = metadata.len();
        
        // Delete the file
        match fs::remove_file(&full_path).await {
            Ok(_) => Ok(format!(
                "Successfully deleted file '{}' ({} bytes)",
                path, file_size
            )),
            Err(e) => Err(AgentError::ToolExecutionError {
                tool: "delete_file".to_string(),
                error: format!("Failed to delete file: {}", e),
            }),
        }
    }
}