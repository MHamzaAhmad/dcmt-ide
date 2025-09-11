use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for reading file contents from the workspace
pub struct ReadFileTool;

#[async_trait]
impl AgentTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
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
    
    async fn execute(&self, workspace: &Path, args: Value, _repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String> {
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
            Err(e) => Err(AgentError::IoError(e)),
        }
    }
}