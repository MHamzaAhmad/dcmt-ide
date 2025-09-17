use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{PathBuf, Path};
use regex::RegexBuilder;
use tokio::fs;

// Mirrored DTOs for search_files tool (desktop)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_true")] 
    pub case_sensitive: bool,
    #[serde(default)]
    pub include_globs: Option<Vec<String>>,
    #[serde(default)]
    pub exclude_globs: Option<Vec<String>>,
    #[serde(default = "default_max_results")] 
    pub max_results: usize,
    #[serde(default)]
    pub context_before: Option<usize>,
    #[serde(default)]
    pub context_after: Option<usize>,
}

fn default_true() -> bool { true }
fn default_max_results() -> usize { 200 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub r#match: String,
    #[serde(default)]
    pub lines_before: Option<Vec<String>>,
    #[serde(default)]
    pub lines_after: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct SearchFilesTool;

impl SearchFilesTool {
    fn matches_file(path: &Path, include: &Option<Vec<String>>, exclude: &Option<Vec<String>>) -> bool {
        let p = path.to_string_lossy();
        if let Some(ex) = exclude {
            if ex.iter().any(|g| p.contains(g)) { return false; }
        }
        if let Some(inc) = include {
            return inc.iter().any(|g| p.contains(g));
        }
        true
    }
}

impl AgentTool for SearchFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_files".to_string(),
                description: "Search files in the workspace quickly with optional regex and globs. Returns line/column positions and context.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "regex": { "type": "boolean" },
                        "case_sensitive": { "type": "boolean" },
                        "include_globs": { "type": "array", "items": {"type": "string"} },
                        "exclude_globs": { "type": "array", "items": {"type": "string"} },
                        "max_results": { "type": "number" },
                        "context_before": { "type": "number" },
                        "context_after": { "type": "number" }
                    },
                    "required": ["query"]
                }),
                display_name: Some("Search Files".to_string()),
                progressive_form: Some("Searching files".to_string()),
            },
        }
    }

    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let a: SearchArgs = serde_json::from_value(args).map_err(|e| AgentError::InvalidToolArguments { tool: "search_files".into(), error: e.to_string() })?;
        let root = validate_workspace_path(workspace_path, ".")?;

        let mut results: Vec<SearchMatch> = Vec::new();

        let regex = if a.regex {
            Some(RegexBuilder::new(&a.query).case_insensitive(!a.case_sensitive).build().map_err(|e| AgentError::ToolExecutionError { tool: "search_files".into(), error: format!("Invalid regex: {}", e) })?)
        } else { None };

        let mut dirs = vec![root.clone()];
        while let Some(dir) = dirs.pop() {
            let mut rd = match fs::read_dir(&dir).await { Ok(rd) => rd, Err(_) => continue };
            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                // Skip hidden/.git dirs
                if path.file_name().and_then(|s| s.to_str()).map(|s| s.starts_with('.')).unwrap_or(false) { continue; }
                if path.is_dir() { dirs.push(path); continue; }
                if !Self::matches_file(&path, &a.include_globs, &a.exclude_globs) { continue; }
                let Ok(content) = fs::read_to_string(&path).await else { continue }; // skip binary
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if results.len() >= a.max_results { break; }
                    let found = if let Some(re) = &regex { re.find(line).map(|m| (m.start(), m.as_str().to_string())) } else { line.find(&a.query).map(|idx| (idx, a.query.clone())) };
                    if let Some((idx, m)) = found {
                        let before = a.context_before.unwrap_or(0);
                        let after = a.context_after.unwrap_or(0);
                        let start = i.saturating_sub(before);
                        let end = (i + 1 + after).min(lines.len());
                        results.push(SearchMatch {
                            path: path.strip_prefix(&root).unwrap_or(&path).to_string_lossy().to_string(),
                            line: i + 1,
                            col: idx + 1,
                            r#match: m,
                            lines_before: if before > 0 { Some(lines[start..i].iter().map(|s| (*s).to_string()).collect()) } else { None },
                            lines_after: if after > 0 { Some(lines[i+1..end].iter().map(|s| (*s).to_string()).collect()) } else { None },
                        });
                    }
                }
            }
            if results.len() >= a.max_results { break; }
        }

        Ok(serde_json::to_string(&results).unwrap_or_else(|_| "[]".into()))
    }
}
