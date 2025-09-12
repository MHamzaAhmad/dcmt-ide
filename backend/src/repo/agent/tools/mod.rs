use async_trait::async_trait;
use std::path::Path;
use serde_json::Value;
use crate::model::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};

// Import individual tools
pub mod read_file;
pub mod write_file;
pub mod update_file;
pub mod list_files;
pub mod create_directory;
pub mod delete_file;
pub mod compile_latex;

pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
pub use update_file::UpdateFileTool;
pub use list_files::ListFilesTool;
pub use create_directory::CreateDirectoryTool;
pub use delete_file::DeleteFileTool;
pub use compile_latex::CompileLatexTool;

/// Base trait that all agent tools must implement
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Returns the name of the tool
    fn name(&self) -> &str;
    
    /// Returns the display name for UI (e.g., "List Files")
    fn display_name(&self) -> &str;
    
    /// Returns the progressive form for UI (e.g., "Listing files")
    fn progressive_form(&self) -> &str;
    
    /// Returns the OpenAI tool definition for this tool
    fn definition(&self) -> ToolDefinition;
    
    /// Executes the tool with the given arguments and repo access
    async fn execute(&self, workspace: &Path, args: Value, repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String>;
}

/// Registry that manages all available tools
pub struct ToolRegistry {
    tools: Vec<Box<dyn AgentTool>>,
}

impl ToolRegistry {
    /// Creates a new tool registry with all available tools
    pub fn new() -> Self {
        let tools: Vec<Box<dyn AgentTool>> = vec![
            Box::new(ReadFileTool),
            Box::new(WriteFileTool),
            Box::new(UpdateFileTool),
            Box::new(ListFilesTool),
            Box::new(CreateDirectoryTool),
            Box::new(DeleteFileTool),
            Box::new(CompileLatexTool),
        ];
        
        Self { tools }
    }
    
    /// Returns OpenAI tool definitions for all registered tools
    pub fn get_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|tool| tool.definition()).collect()
    }
    
    /// Executes a tool by name with the given arguments
    pub async fn execute(&self, name: &str, workspace: &Path, args: Value, repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String> {
        let tool = self.tools
            .iter()
            .find(|tool| tool.name() == name)
            .ok_or_else(|| AgentError::ToolNotFound { name: name.to_string() })?;
            
        tool.execute(workspace, args, repo).await.map_err(|e| {
            AgentError::ToolExecutionError {
                tool: name.to_string(),
                error: e.to_string(),
            }
        })
    }
    
    /// Returns a list of all tool names
    pub fn get_tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|tool| tool.name()).collect()
    }
    
    /// Returns the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
    
    /// Gets tool UI metadata by tool name
    pub fn get_tool_metadata(&self, name: &str) -> Option<(String, String)> {
        self.tools
            .iter()
            .find(|tool| tool.name() == name)
            .map(|tool| (tool.display_name().to_string(), tool.progressive_form().to_string()))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}