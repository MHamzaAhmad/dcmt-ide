use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for creating or completely overwriting files in the workspace
pub struct WriteFileTool;

#[async_trait]
impl AgentTool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Create a new file or completely overwrite an existing file with the provided content. Use this for creating new LaTeX documents, configuration files, or completely replacing file contents.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace (e.g., 'main.tex', 'bibliography.bib')"
                        },
                        "content": {
                            "type": "string",
                            "description": "Complete content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
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
            
        let content = args["content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: self.name().to_string(), 
                error: "Missing 'content' parameter".to_string() 
            })?;
        
        // Ensure the path is relative and doesn't contain dangerous patterns
        if path.starts_with('/') || path.contains("..") {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Path must be relative and cannot contain '..' for security reasons".to_string(),
            });
        }
        
        let full_path = workspace.join(path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    AgentError::IoError(e)
                })?;
            }
        }
        
        // Check if file already exists to provide appropriate feedback
        let file_exists = full_path.exists();
        
        match fs::write(&full_path, content).await {
            Ok(()) => {
                let action = if file_exists { "overwritten" } else { "created" };
                let byte_count = content.len();
                let line_count = content.lines().count();
                
                Ok(format!(
                    "Successfully {} file '{}' ({} bytes, {} lines)",
                    action, path, byte_count, line_count
                ))
            }
            Err(e) => Err(AgentError::IoError(e)),
        }
    }
}