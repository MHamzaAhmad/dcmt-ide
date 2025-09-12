use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for writing/creating files in the workspace
#[derive(Clone)]
pub struct WriteFileTool;

impl AgentTool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "write_file".to_string(),
                description: "Create a new file or overwrite an existing file in the workspace. Use this tool to create LaTeX documents, configuration files, or any other text files needed for the project.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace (e.g., 'main.tex', 'chapters/introduction.tex')"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }),
                display_name: Some("Write File".to_string()),
                progressive_form: Some("Writing file".to_string()),
            },
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "write_file".to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
            
        let content = args["content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "write_file".to_string(), 
                error: "Missing 'content' parameter".to_string() 
            })?;
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| AgentError::ToolExecutionError {
                        tool: "write_file".to_string(),
                        error: format!("Failed to create parent directories: {}", e),
                    })?;
            }
        }
        
        // Write the file
        match fs::write(&full_path, content).await {
            Ok(_) => {
                let file_size = content.len();
                let line_count = content.lines().count();
                
                Ok(format!(
                    "Successfully wrote file '{}' ({} bytes, {} lines)",
                    path, file_size, line_count
                ))
            }
            Err(e) => Err(AgentError::ToolExecutionError {
                tool: "write_file".to_string(),
                error: format!("Failed to write file: {}", e),
            }),
        }
    }
}