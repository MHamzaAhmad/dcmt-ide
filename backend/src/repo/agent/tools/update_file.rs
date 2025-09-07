use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for updating specific parts of files using find-and-replace operations
pub struct UpdateFileTool;

#[async_trait]
impl AgentTool for UpdateFileTool {
    fn name(&self) -> &str {
        "update_file"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Update specific parts of an existing file by replacing old content with new content. Use this for making targeted modifications to LaTeX files while preserving the rest of the content.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within workspace"
                        },
                        "old_content": {
                            "type": "string",
                            "description": "Exact content to find and replace (must match exactly including whitespace)"
                        },
                        "new_content": {
                            "type": "string",
                            "description": "New content to replace the old content with"
                        }
                    },
                    "required": ["path", "old_content", "new_content"]
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
            
        let old_content = args["old_content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: self.name().to_string(), 
                error: "Missing 'old_content' parameter".to_string() 
            })?;
            
        let new_content = args["new_content"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: self.name().to_string(), 
                error: "Missing 'new_content' parameter".to_string() 
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
        
        // Read the current file content
        let current_content = fs::read_to_string(&full_path).await
            .map_err(AgentError::IoError)?;
        
        // Check if the old content exists in the file
        if !current_content.contains(old_content) {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: format!(
                    "Could not find the specified content to replace in file '{}'. Make sure the old_content matches exactly including all whitespace and line breaks.", 
                    path
                ),
            });
        }
        
        // Count occurrences to warn about multiple matches
        let occurrence_count = current_content.matches(old_content).count();
        if occurrence_count > 1 {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: format!(
                    "Found {} occurrences of the content to replace in file '{}'. For safety, the content to replace must be unique. Please be more specific with the old_content parameter.", 
                    occurrence_count, path
                ),
            });
        }
        
        // Perform the replacement
        let updated_content = current_content.replace(old_content, new_content);
        
        // Write the updated content back to the file
        match fs::write(&full_path, &updated_content).await {
            Ok(()) => {
                let old_lines = old_content.lines().count();
                let new_lines = new_content.lines().count();
                let total_lines = updated_content.lines().count();
                
                Ok(format!(
                    "Successfully updated file '{}'. Replaced {} line(s) with {} line(s). File now has {} total lines.",
                    path, old_lines, new_lines, total_lines
                ))
            }
            Err(e) => Err(AgentError::IoError(e)),
        }
    }
}