use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for listing files and directories in the workspace
#[derive(Clone)]
pub struct ListFilesTool;

impl AgentTool for ListFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_files".to_string(),
                description: "List files and directories in a specified path within the workspace. Use this tool to explore project structure, understand directory organization, or find specific files.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the directory within workspace (use '.' for root workspace directory)",
                            "default": "."
                        },
                        "show_hidden": {
                            "type": "boolean",
                            "description": "Whether to show hidden files and directories (starting with '.')",
                            "default": false
                        }
                    },
                    "required": []
                }),
                display_name: Some("List Files".to_string()),
                progressive_form: Some("Listing files".to_string()),
            },
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .unwrap_or(".");
            
        let show_hidden = args["show_hidden"]
            .as_bool()
            .unwrap_or(false);
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Check if path exists
        if !full_path.exists() {
            return Err(AgentError::ToolExecutionError {
                tool: "list_files".to_string(),
                error: format!("Path not found: {}", path),
            });
        }
        
        // Check if it's a directory
        if !full_path.is_dir() {
            return Err(AgentError::ToolExecutionError {
                tool: "list_files".to_string(),
                error: format!("Path '{}' is not a directory", path),
            });
        }
        
        // Read directory contents
        let mut entries = fs::read_dir(&full_path).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: "list_files".to_string(),
                error: format!("Failed to read directory: {}", e),
            })?;
        
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: "list_files".to_string(),
                error: format!("Failed to read directory entry: {}", e),
            })? {
            
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy().to_string();
            
            // Skip hidden files if not requested
            if !show_hidden && file_name_str.starts_with('.') {
                continue;
            }
            
            let metadata = entry.metadata().await
                .map_err(|e| AgentError::ToolExecutionError {
                    tool: "list_files".to_string(),
                    error: format!("Failed to read file metadata: {}", e),
                })?;
            
            if metadata.is_dir() {
                directories.push(format!("{}/", file_name_str));
            } else {
                let size = metadata.len();
                files.push(format!("{} ({} bytes)", file_name_str, size));
            }
        }
        
        // Sort entries
        directories.sort();
        files.sort();
        
        // Build result string
        let mut result = format!("Contents of '{}':\n\n", path);
        
        if !directories.is_empty() {
            result.push_str("Directories:\n");
            for dir in directories {
                result.push_str(&format!("  {}\n", dir));
            }
            result.push('\n');
        }
        
        if !files.is_empty() {
            result.push_str("Files:\n");
            for file in files {
                result.push_str(&format!("  {}\n", file));
            }
        }
        
        if result.ends_with('\n') {
            result.pop();
        }
        
        if result == format!("Contents of '{}':\n\n", path) {
            result.push_str("(empty directory)");
        }
        
        Ok(result)
    }
}