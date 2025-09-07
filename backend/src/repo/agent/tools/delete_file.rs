use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for deleting files from the workspace
pub struct DeleteFileTool;

#[async_trait]
impl AgentTool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Delete a file from the workspace. Use this carefully to remove unwanted files or clean up the project structure. Cannot be undone.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file to delete"
                        },
                        "confirm": {
                            "type": "boolean",
                            "description": "Confirmation that you want to delete the file (must be true)",
                            "default": false
                        }
                    },
                    "required": ["path", "confirm"]
                }),
            },
        }
    }
    
    async fn execute(&self, workspace: &Path, args: Value) -> AgentResult<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: self.name().to_string(), 
                error: "Missing 'path' parameter".to_string() 
            })?;
            
        let confirm = args["confirm"]
            .as_bool()
            .unwrap_or(false);
        
        // Require explicit confirmation for safety
        if !confirm {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "File deletion requires explicit confirmation. Set 'confirm' parameter to true.".to_string(),
            });
        }
        
        // Ensure the path is relative and doesn't contain dangerous patterns
        if path.starts_with('/') || path.contains("..") {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Path must be relative and cannot contain '..' for security reasons".to_string(),
            });
        }
        
        // Don't allow deleting critical patterns
        let dangerous_patterns = [
            ".",
            "./",
            "",
        ];
        
        if dangerous_patterns.contains(&path) {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Cannot delete root directory or invalid paths".to_string(),
            });
        }
        
        let full_path = workspace.join(path);
        
        // Check if file exists
        if !full_path.exists() {
            return Err(AgentError::IoError(
                std::io::Error::new(
                    std::io::ErrorKind::NotFound, 
                    format!("File not found: {}", path)
                )
            ));
        }
        
        // Check if it's actually a file (not a directory)
        if !full_path.is_file() {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: format!("Path '{}' is not a file. Use a different method to delete directories.", path),
            });
        }
        
        // Get file info before deletion for reporting
        let metadata = fs::metadata(&full_path).await
            .map_err(AgentError::IoError)?;
        let file_size = metadata.len();
        
        // Perform the deletion
        match fs::remove_file(&full_path).await {
            Ok(()) => {
                Ok(format!("Successfully deleted file '{}' ({} bytes)", path, file_size))
            }
            Err(e) => Err(AgentError::IoError(e)),
        }
    }
}