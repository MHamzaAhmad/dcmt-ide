use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncReadExt;

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
        
        // Enforce single LaTeX main per workspace in a simple way:
        // If writing a .tex file with \\documentclass and an existing main already exists elsewhere, block.
        if path.ends_with(".tex") && content.contains("\\documentclass") {
            let full_target = validate_workspace_path(workspace_path, path)?;
            if let Some(existing_main) = find_existing_main_with_documentclass(workspace_path, Some(&full_target)).await.map_err(|e| AgentError::ToolExecutionError { tool: "write_file".to_string(), error: e.to_string() })? {
                let existing_rel = existing_main
                    .strip_prefix(workspace_path)
                    .unwrap_or(&existing_main)
                    .to_string_lossy()
                    .to_string();
                if existing_rel != path {
                    return Err(AgentError::InvalidToolArguments {
                        tool: "write_file".to_string(),
                        error: format!(
                            "A LaTeX main file already exists at '{}'. Strictly one LaTeX project per workspace. Update the existing main or delete it before creating a new one.",
                            existing_rel
                        ),
                    });
                }
            }
        }

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

// Iteratively scan for a .tex file containing \\documentclass in the workspace, skipping a target path if provided.
async fn find_existing_main_with_documentclass(workspace: &PathBuf, skip: Option<&PathBuf>) -> std::io::Result<Option<std::path::PathBuf>> {
    let mut stack = vec![workspace.clone()];
    while let Some(dir) = stack.pop() {
        let mut rd = fs::read_dir(&dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let p = entry.path();
            if let Some(skip_p) = skip { if &p == skip_p { continue; } }
            if p.is_dir() {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if matches!(name, "build" | "dist" | "output" | ".git" | "node_modules" | "target") {
                        continue;
                    }
                }
                stack.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("tex") {
                let file = fs::File::open(&p).await?;
                let mut buf = String::new();
                let _ = tokio::io::BufReader::new(file).take(128 * 1024).read_to_string(&mut buf).await?;
                if buf.contains("\\documentclass") {
                    return Ok(Some(p));
                }
            }
        }
    }
    Ok(None)
}