use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for reading file contents from the workspace
#[derive(Clone)]
pub struct ReadFileTool;

impl AgentTool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file from the workspace. Use this tool to examine existing files, understand project structure, or read LaTeX documents before making modifications.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace (e.g., 'main.tex', 'chapters/introduction.tex')"
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
                tool: "read_file".to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Check if file exists
        if !full_path.exists() {
            return Err(AgentError::ToolExecutionError {
                tool: "read_file".to_string(),
                error: format!("File not found: {}", path),
            });
        }
        
        // Check if it's actually a file (not a directory)
        if !full_path.is_file() {
            return Err(AgentError::ToolExecutionError {
                tool: "read_file".to_string(),
                error: format!("Path '{}' is not a file", path),
            });
        }
        
        match fs::read_to_string(&full_path).await {
            Ok(content) => {
                let file_size = content.len();
                let line_count = content.lines().count();
                
                Ok(format!(
                    "Successfully read file '{}' ({} bytes, {} lines):\n\n{}",
                    path, file_size, line_count, content
                ))
            }
            Err(e) => Err(AgentError::ToolExecutionError {
                tool: "read_file".to_string(),
                error: format!("Failed to read file: {}", e),
            }),
        }
    }
}