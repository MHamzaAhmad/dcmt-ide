use std::path::PathBuf;
use serde_json::Value;
use crate::models::agent::{AgentResult, ToolDefinition, AgentError};

pub mod read_file;
pub mod write_file;
pub mod update_file;
pub mod list_files;
pub mod create_directory;
pub mod delete_file;

pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
pub use update_file::UpdateFileTool;
pub use list_files::ListFilesTool;
pub use create_directory::CreateDirectoryTool;
pub use delete_file::DeleteFileTool;

/// Trait that all agent tools must implement
pub trait AgentTool: Send + Sync {
    /// Returns the function definition for this tool (used by LLM)
    fn definition(&self) -> ToolDefinition;
    
    /// Executes the tool with given workspace path and arguments
    async fn execute(&self, workspace_path: &PathBuf, args: Value) -> AgentResult<String>;
}

/// Enum containing all available tools
#[derive(Clone)]
pub enum Tool {
    ReadFile(ReadFileTool),
    WriteFile(WriteFileTool),
    UpdateFile(UpdateFileTool),
    ListFiles(ListFilesTool),
    CreateDirectory(CreateDirectoryTool),
    DeleteFile(DeleteFileTool),
}

impl Tool {
    fn definition(&self) -> ToolDefinition {
        match self {
            Tool::ReadFile(tool) => tool.definition(),
            Tool::WriteFile(tool) => tool.definition(),
            Tool::UpdateFile(tool) => tool.definition(),
            Tool::ListFiles(tool) => tool.definition(),
            Tool::CreateDirectory(tool) => tool.definition(),
            Tool::DeleteFile(tool) => tool.definition(),
        }
    }
    
    async fn execute(&self, workspace_path: &PathBuf, args: Value) -> AgentResult<String> {
        match self {
            Tool::ReadFile(tool) => tool.execute(workspace_path, args).await,
            Tool::WriteFile(tool) => tool.execute(workspace_path, args).await,
            Tool::UpdateFile(tool) => tool.execute(workspace_path, args).await,
            Tool::ListFiles(tool) => tool.execute(workspace_path, args).await,
            Tool::CreateDirectory(tool) => tool.execute(workspace_path, args).await,
            Tool::DeleteFile(tool) => tool.execute(workspace_path, args).await,
        }
    }
    
    fn name(&self) -> &str {
        match self {
            Tool::ReadFile(_) => "read_file",
            Tool::WriteFile(_) => "write_file",
            Tool::UpdateFile(_) => "update_file",
            Tool::ListFiles(_) => "list_files",
            Tool::CreateDirectory(_) => "create_directory",
            Tool::DeleteFile(_) => "delete_file",
        }
    }
}

/// Registry of all available agent tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Vec<Tool>,
}

impl ToolRegistry {
    /// Creates a new tool registry with all available tools
    pub fn new() -> Self {
        let tools = vec![
            Tool::ReadFile(ReadFileTool),
            Tool::WriteFile(WriteFileTool),
            Tool::UpdateFile(UpdateFileTool),
            Tool::ListFiles(ListFilesTool),
            Tool::CreateDirectory(CreateDirectoryTool),
            Tool::DeleteFile(DeleteFileTool),
        ];
        
        Self { tools }
    }
    
    /// Gets tool definitions for all registered tools (for LLM)
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|tool| tool.definition()).collect()
    }
    
    /// Executes a tool by name with given workspace and arguments
    pub async fn execute(&self, tool_name: &str, workspace_path: &PathBuf, args: Value) -> AgentResult<String> {
        // Find the tool by name
        let tool = self.tools.iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| AgentError::ToolExecutionError {
                tool: tool_name.to_string(),
                error: "Tool not found".to_string(),
            })?;
        
        // Execute the tool
        match tool.execute(workspace_path, args).await {
            Ok(result) => Ok(result),
            Err(e) => Err(AgentError::ToolExecutionError {
                tool: tool_name.to_string(),
                error: e.to_string(),
            })
        }
    }
    
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to validate that a path is within the workspace
pub fn validate_workspace_path(workspace_path: &PathBuf, target_path: &str) -> AgentResult<PathBuf> {
    // Remove any leading slash or make relative
    let clean_target = target_path.trim_start_matches('/');
    
    // Build the full path
    let full_path = workspace_path.join(clean_target);
    
    // Canonicalize both paths to resolve any .. or . components
    let canonical_workspace = workspace_path.canonicalize()
        .map_err(|e| AgentError::Generic(anyhow::anyhow!("Invalid workspace path: {}", e)))?;
    
    let canonical_target = match full_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If the path doesn't exist yet, check if its parent directory is within workspace
            if let Some(parent) = full_path.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize()
                        .map_err(|e| AgentError::Generic(anyhow::anyhow!("Invalid parent path: {}", e)))?;
                    
                    if !canonical_parent.starts_with(&canonical_workspace) {
                        return Err(AgentError::Generic(anyhow::anyhow!(
                            "Path traversal attempt detected: {} is outside workspace",
                            target_path
                        )));
                    }
                }
            }
            full_path.clone()
        }
    };
    
    // Check if the canonical target path is within the workspace
    if canonical_target.starts_with(&canonical_workspace) {
        Ok(full_path)
    } else {
        Err(AgentError::Generic(anyhow::anyhow!(
            "Path traversal attempt detected: {} is outside workspace",
            target_path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.tools.len(), 6);
        
        let tool_names: Vec<String> = registry.tools.iter()
            .map(|tool| tool.name().to_string())
            .collect();
        assert!(tool_names.contains(&"read_file".to_string()));
        assert!(tool_names.contains(&"write_file".to_string()));
        assert!(tool_names.contains(&"update_file".to_string()));
        assert!(tool_names.contains(&"list_files".to_string()));
        assert!(tool_names.contains(&"create_directory".to_string()));
        assert!(tool_names.contains(&"delete_file".to_string()));
    }
    
    #[test]
    fn test_path_validation() {
        let temp_dir = std::env::temp_dir().join("test_workspace");
        fs::create_dir_all(&temp_dir).unwrap();
        
        // Valid path within workspace
        let valid_result = validate_workspace_path(&temp_dir, "subdir/file.txt");
        assert!(valid_result.is_ok());
        
        // Invalid path traversal attempt
        let invalid_result = validate_workspace_path(&temp_dir, "../outside_workspace/file.txt");
        assert!(invalid_result.is_err());
        
        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}