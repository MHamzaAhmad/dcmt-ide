use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Tool for updating files using find and replace
#[derive(Clone)]
pub struct UpdateFileTool;

impl AgentTool for UpdateFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "update_file".to_string(),
                description: "Update a file by finding and replacing specific content. This tool performs exact text matching and replacement. Use this for targeted modifications to existing files.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace (e.g., 'main.tex', 'chapters/introduction.tex')"
                        },
                        "old_content": {
                            "type": "string",
                            "description": "The exact content to find and replace (must match exactly, including whitespace and newlines)"
                        },
                        "new_content": {
                            "type": "string",
                            "description": "The new content to replace the old content with"
                        }
                    },
                    "required": ["path", "old_content", "new_content"]
                }),
                display_name: Some("Update File".to_string()),
                progressive_form: Some("Updating file".to_string()),
            },
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "update_file".to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
            
        let old_content = args["old_content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "update_file".to_string(), 
                error: "Missing 'old_content' parameter".to_string() 
            })?;
            
        let new_content = args["new_content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: "update_file".to_string(), 
                error: "Missing 'new_content' parameter".to_string() 
            })?;
        
        // Validate and get the full path
        let full_path = validate_workspace_path(workspace_path, path)?;
        
        // Check if file exists
        if !full_path.exists() {
            return Err(AgentError::ToolExecutionError {
                tool: "update_file".to_string(),
                error: format!("File not found: {}", path),
            });
        }
        
        // Read the current file content
        let current_content = fs::read_to_string(&full_path).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: "update_file".to_string(),
                error: format!("Failed to read file: {}", e),
            })?;
        
        // Check if old_content exists in the file
        if !current_content.contains(old_content) {
            return Err(AgentError::ToolExecutionError {
                tool: "update_file".to_string(),
                error: format!("Old content not found in file: {}", path),
            });
        }
        
        // Count occurrences to ensure it's unique
        let occurrences = current_content.matches(old_content).count();
        if occurrences > 1 {
            return Err(AgentError::ToolExecutionError {
                tool: "update_file".to_string(),
                error: format!("Old content appears {} times in file. Content must be unique for safe replacement.", occurrences),
            });
        }
        
        // Perform the replacement
        let updated_content = current_content.replace(old_content, new_content);
        
        // Write the updated content back to the file
        fs::write(&full_path, &updated_content).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: "update_file".to_string(),
                error: format!("Failed to write updated file: {}", e),
            })?;
        
        let old_lines = old_content.lines().count();
        let new_lines = new_content.lines().count();
        let total_lines = updated_content.lines().count();
        
        Ok(format!(
            "Successfully updated file '{}': replaced {} lines with {} lines (total file now {} lines)",
            path, old_lines, new_lines, total_lines
        ))
    }
}