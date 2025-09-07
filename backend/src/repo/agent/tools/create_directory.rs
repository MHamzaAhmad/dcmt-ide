use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for creating directories in the workspace
pub struct CreateDirectoryTool;

#[async_trait]
impl AgentTool for CreateDirectoryTool {
    fn name(&self) -> &str {
        "create_directory"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Create a new directory in the workspace. Use this to organize LaTeX projects with proper directory structure (e.g., chapters/, figures/, bibliography/).".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the directory to create (e.g., 'chapters', 'figures/diagrams')"
                        }
                    },
                    "required": ["path"]
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
        
        // Ensure the path is relative and doesn't contain dangerous patterns
        if path.starts_with('/') || path.contains("..") {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Path must be relative and cannot contain '..' for security reasons".to_string(),
            });
        }
        
        // Don't allow creating the root directory or empty path
        if path.is_empty() || path == "." || path == "./" {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Cannot create root directory or empty path".to_string(),
            });
        }
        
        let full_path = workspace.join(path);
        
        // Check if directory already exists
        if full_path.exists() {
            if full_path.is_dir() {
                return Ok(format!("Directory '{}' already exists", path));
            } else {
                return Err(AgentError::InvalidToolArguments {
                    tool: self.name().to_string(),
                    error: format!("Path '{}' exists but is not a directory", path),
                });
            }
        }
        
        // Create the directory and any parent directories
        match fs::create_dir_all(&full_path).await {
            Ok(()) => {
                Ok(format!("Successfully created directory '{}'", path))
            }
            Err(e) => Err(AgentError::IoError(e)),
        }
    }
}