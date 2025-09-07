use super::*;
use tokio::fs;
use std::path::Path;

/// Tool for listing files and directories in the workspace
pub struct ListFilesTool;

#[async_trait]
impl AgentTool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "List files and directories in a specified path within the workspace. Use this to explore the project structure, find existing files, or understand the organization of a LaTeX project.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the directory to list (use '.' for workspace root)",
                            "default": "."
                        },
                        "show_hidden": {
                            "type": "boolean",
                            "description": "Whether to include hidden files and directories (starting with '.')",
                            "default": false
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
            .unwrap_or(".");
            
        let show_hidden = args["show_hidden"]
            .as_bool()
            .unwrap_or(false);
        
        // Ensure the path is relative and doesn't contain dangerous patterns
        if path.starts_with('/') || path.contains("..") {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Path must be relative and cannot contain '..' for security reasons".to_string(),
            });
        }
        
        let full_path = workspace.join(path);
        
        // Check if path exists
        if !full_path.exists() {
            return Err(AgentError::IoError(
                std::io::Error::new(
                    std::io::ErrorKind::NotFound, 
                    format!("Directory not found: {}", path)
                )
            ));
        }
        
        // Check if it's actually a directory
        if !full_path.is_dir() {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: format!("Path '{}' is not a directory", path),
            });
        }
        
        let mut entries = fs::read_dir(&full_path).await
            .map_err(AgentError::IoError)?;
        
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        while let Some(entry) = entries.next_entry().await.map_err(AgentError::IoError)? {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            
            // Skip hidden files unless requested
            if !show_hidden && name_str.starts_with('.') {
                continue;
            }
            
            let metadata = entry.metadata().await.map_err(AgentError::IoError)?;
            
            if metadata.is_dir() {
                directories.push(format!("{}/", name_str));
            } else {
                let size = metadata.len();
                files.push(format!("{} ({} bytes)", name_str, size));
            }
        }
        
        // Sort entries
        directories.sort();
        files.sort();
        
        let mut result = Vec::new();
        
        // Add header
        let display_path = if path == "." { "workspace root" } else { path };
        result.push(format!("Contents of '{}':", display_path));
        result.push("".to_string());
        
        // Add directories first
        if !directories.is_empty() {
            result.push("Directories:".to_string());
            for dir in &directories {
                result.push(format!("  📁 {}", dir));
            }
            result.push("".to_string());
        }
        
        // Add files
        if !files.is_empty() {
            result.push("Files:".to_string());
            for file in &files {
                result.push(format!("  📄 {}", file));
            }
        }
        
        if directories.is_empty() && files.is_empty() {
            result.push("(Directory is empty)".to_string());
        } else {
            result.push("".to_string());
            result.push(format!("Total: {} directories, {} files", directories.len(), files.len()));
        }
        
        Ok(result.join("\n"))
    }
}