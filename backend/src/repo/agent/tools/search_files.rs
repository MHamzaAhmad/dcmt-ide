use super::*;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_true")] pub case_sensitive: bool,
    #[serde(default)] pub include_globs: Option<Vec<String>>,
    #[serde(default)] pub exclude_globs: Option<Vec<String>>,
    #[serde(default = "default_max")] pub max_results: usize,
    #[serde(default)] pub context_before: Option<usize>,
    #[serde(default)] pub context_after: Option<usize>,
}
fn default_true() -> bool { true }
fn default_max() -> usize { 200 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch { pub path: String, pub line: usize, pub col: usize, pub r#match: String, pub lines_before: Option<Vec<String>>, pub lines_after: Option<Vec<String>> }

pub struct SearchFilesTool;

impl SearchFilesTool {
    fn matches(path: &std::path::Path, include: &Option<Vec<String>>, exclude: &Option<Vec<String>>) -> bool {
        let p = path.to_string_lossy();
        if let Some(ex) = exclude { if ex.iter().any(|g| p.contains(g)) { return false; } }
        if let Some(inc) = include { return inc.iter().any(|g| p.contains(g)); }
        true
    }
}

#[async_trait::async_trait]
impl super::AgentTool for SearchFilesTool {
    fn name(&self) -> &str { "search_files" }
    fn display_name(&self) -> &str { "Search Files" }
    fn progressive_form(&self) -> &str { "Searching files" }
    fn definition(&self) -> ToolDefinition {
        ToolDefinition { tool_type: "function".into(), function: FunctionDefinition {
            name: self.name().into(), description: "Search files in the workspace".into(),
            parameters: serde_json::json!({
                "type": "object", "properties": {
                    "query": {"type":"string"},
                    "regex": {"type":"boolean"},
                    "case_sensitive": {"type":"boolean"},
                    "include_globs": {"type":"array","items":{"type":"string"}},
                    "exclude_globs": {"type":"array","items":{"type":"string"}},
                    "max_results": {"type":"number"},
                    "context_before": {"type":"number"},
                    "context_after": {"type":"number"}
                }, "required": ["query"]
            }),
            display_name: Some(self.display_name().into()),
            progressive_form: Some(self.progressive_form().into()),
        }}
    }
    async fn execute(&self, workspace: &Path, args: serde_json::Value, _repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String> {
        let a: SearchArgs = serde_json::from_value(args).map_err(|e| AgentError::InvalidToolArguments { tool: self.name().into(), error: e.to_string() })?;
        let mut results: Vec<SearchMatch> = Vec::new();
        let regex = if a.regex { Some(RegexBuilder::new(&a.query).case_insensitive(!a.case_sensitive).build().map_err(|e| AgentError::ToolExecutionError { tool: self.name().into(), error: format!("Invalid regex: {}", e) })?) } else { None };

        let mut stack = vec![workspace.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let rd = match tokio::fs::read_dir(&dir).await { Ok(rd) => rd, Err(_) => continue };
            tokio::pin!(rd);
            let mut rd = rd;
            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                if path.file_name().and_then(|s| s.to_str()).map(|s| s.starts_with('.')).unwrap_or(false) { continue; }
                if path.is_dir() { stack.push(path); continue; }
                if !Self::matches(&path, &a.include_globs, &a.exclude_globs) { continue; }
                let Ok(content) = tokio::fs::read_to_string(&path).await else { continue };
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
                            path: path.strip_prefix(workspace).unwrap_or(&path).to_string_lossy().to_string(),
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
