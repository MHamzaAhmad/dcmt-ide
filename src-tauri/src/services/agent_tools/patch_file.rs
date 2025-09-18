use super::{AgentTool, validate_workspace_path};
use crate::models::agent::{ToolDefinition, FunctionDefinition, AgentResult, AgentError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// Mirrored DTOs for patch_file tool (desktop)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PatchOp {
    ReplaceRange { start_line: usize, start_col: usize, end_line: usize, end_col: usize, text: String },
    InsertAt { line: usize, col: usize, text: String },
    DeleteRange { start_line: usize, start_col: usize, end_line: usize, end_col: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPrecondition {
    #[serde(default)]
    pub mtime: Option<u64>,
    #[serde(default)]
    pub on_mismatch: Option<String>, // "fail" | "rebase" (rebase not implemented yet)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatchPayload {
    Ops { ops: Vec<PatchOp> },
    UnifiedDiff { unified_diff: String },
    DmpOps { dmp_ops: Value },
}

#[derive(Clone)]
pub struct PatchFileTool;

impl PatchFileTool {
    fn apply_ops(text: &str, ops: &[PatchOp]) -> Result<String, String> {
        // 1-based line/col to byte indices; apply ops in a stable order to avoid index drift
        // Strategy: convert to lines vec, manipulate per op, then join
        let mut lines: Vec<String> = text.split_inclusive('\n').map(|s| s.to_string()).collect();
        if lines.is_empty() { lines.push(String::new()); }

    let apply_range = |
            lines: &mut Vec<String>,
            start_line: usize, start_col: usize,
            end_line: usize, end_col: usize,
            replacement: Option<&str>
        | -> Result<(), String> {
            // Clamp indices and convert 1-based to 0-based
            let sl = start_line.saturating_sub(1).min(lines.len() - 1);
            let el = end_line.saturating_sub(1).min(lines.len() - 1);
            let sc = start_col.saturating_sub(1);
            let ec = end_col.saturating_sub(1);

            if sl > el || (sl == el && sc > ec) {
                return Err("Invalid range for patch op".to_string());
            }

            if sl == el {
                let line = &mut lines[sl];
                if sc > line.len() || ec > line.len() { return Err("Range exceeds line length".to_string()); }
                let mut new_line = String::new();
                new_line.push_str(&line[..sc]);
                if let Some(rep) = replacement { new_line.push_str(rep); }
                new_line.push_str(&line[ec..]);
                *line = new_line;
            } else {
                // Multi-line replace/delete
                let first = &lines[sl].clone();
                let last = &lines[el].clone();
                let before = if sc <= first.len() { &first[..sc] } else { &first[..] };
                let after = if ec <= last.len() { &last[ec..] } else { "" };
                let mut merged = String::new();
                merged.push_str(before);
                if let Some(rep) = replacement { merged.push_str(rep); }
                merged.push_str(after);

                // Replace range of lines with merged single line
                lines.splice(sl..=el, std::iter::once(merged));
            }
            Ok(())
        };

        for op in ops {
            match op {
                PatchOp::ReplaceRange { start_line, start_col, end_line, end_col, text } => {
                    apply_range(&mut lines, *start_line, *start_col, *end_line, *end_col, Some(text))?;
                }
                PatchOp::InsertAt { line, col, text } => {
                    apply_range(&mut lines, *line, *col, *line, *col, Some(text))?;
                }
                PatchOp::DeleteRange { start_line, start_col, end_line, end_col } => {
                    apply_range(&mut lines, *start_line, *start_col, *end_line, *end_col, None)?;
                }
            }
        }

        Ok(lines.into_iter().collect())
    }

    // Minimal unified-diff parser and applier for a single file
    fn apply_unified_diff(text: &str, diff: &str) -> Result<String, String> {
        #[derive(Debug)]
        struct Hunk {
            old_start: isize,
            old_len: isize,
            _new_start: isize,
            new_len: isize,
            lines: Vec<(char, String)>,
        }

        let mut hunks: Vec<Hunk> = Vec::new();
        let mut cur_hunk: Option<Hunk> = None;

        for line in diff.lines() {
            if line.starts_with("@@") {
                // Flush previous hunk
                if let Some(h) = cur_hunk.take() { hunks.push(h); }
                // @@ -old_start,old_len +new_start,new_len @@ optional section
                // Extract numbers
                // Remove leading @@ and trailing @@
                let end = line.rfind("@@").ok_or_else(|| "Invalid unified diff hunk header".to_string())?;
                let header = &line[2..end].trim();
                let mut parts = header.split_whitespace();
                let old = parts.next().ok_or_else(|| "Malformed hunk header".to_string())?; // like -12,5 or -12
                let new = parts.next().ok_or_else(|| "Malformed hunk header".to_string())?; // like +12,6 or +12

                let parse_range = |s: &str| -> Result<(isize, isize), String> {
                    let s = s.trim();
                    if !s.starts_with('+') && !s.starts_with('-') { return Err("Bad range token".into()); }
                    let s = &s[1..];
                    let mut it = s.split(',');
                    let start: isize = it.next().ok_or("missing start").and_then(|v| v.parse().map_err(|_| "bad start"))?;
                    let len: isize = it.next().map(|v| v.parse().map_err(|_| "bad len")).transpose()?.unwrap_or(1);
                    Ok((start, len))
                };
                let (old_start, old_len) = parse_range(old)?;
                let (new_start, new_len) = parse_range(new)?;
                cur_hunk = Some(Hunk { old_start, old_len, _new_start: new_start, new_len, lines: Vec::new() });
            } else if line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("diff ") || line.starts_with("index ") {
                // file headers; ignore
                continue;
            } else if let Some(h) = &mut cur_hunk {
                if let Some(prefix) = line.chars().next() {
                    match prefix {
                        ' ' | '-' | '+' => {
                            let content = line[1..].to_string();
                            h.lines.push((prefix, content));
                        }
                        '\\' => {
                            // e.g., "\\ No newline at end of file"; ignore
                        }
                        _ => {
                            // treat as context if unknown prefix
                            h.lines.push((' ', line.to_string()));
                        }
                    }
                }
            }
        }
        if let Some(h) = cur_hunk.take() { hunks.push(h); }

        // Apply hunks in order with running delta
        let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        let mut delta: isize = 0;

        for h in hunks {
            let old_start0 = (h.old_start - 1 + delta).max(0) as usize;
            // Build replacement segment and compute how many old lines are consumed
            let mut seg: Vec<String> = Vec::new();
            let mut consume: usize = 0;
            let mut idx = old_start0;

            for (tag, content) in &h.lines {
                match *tag {
                    ' ' => {
                        // context; must match existing
                        let exist = lines.get(idx).ok_or_else(|| "Context out of range while applying patch".to_string())?;
                        if exist != content { return Err("Context mismatch while applying patch".into()); }
                        seg.push(exist.clone());
                        idx += 1; consume += 1;
                    }
                    '-' => {
                        // deletion; must match existing but not included in seg
                        let exist = lines.get(idx).ok_or_else(|| "Deletion out of range while applying patch".to_string())?;
                        if exist != content { return Err("Deletion mismatch while applying patch".into()); }
                        idx += 1; consume += 1;
                    }
                    '+' => {
                        // insertion; included in seg
                        seg.push(content.clone());
                    }
                    _ => {}
                }
            }

            // Replace range [old_start0, old_start0 + consume) with seg
            if old_start0 > lines.len() { return Err("Patch position out of range".into()); }
            let end = (old_start0 + consume).min(lines.len());
            lines.splice(old_start0..end, seg.into_iter());

            // Update delta for next hunks
            let delta_change = h.new_len - h.old_len;
            delta += delta_change;
        }

        Ok(lines.join("\n"))
    }

    // Apply Google diff-match-patch operations represented as an array of {op, text} or [op, text]
    fn apply_dmp_ops(text: &str, dmp_ops: &Value) -> Result<String, String> {
        let ops = dmp_ops.as_array().ok_or_else(|| "dmp_ops must be an array".to_string())?;
        let mut src = text;
        let mut out = String::new();
        for item in ops {
            let (op_code, seg) = if let Some(arr) = item.as_array() {
                if arr.len() != 2 { return Err("Each dmp op must have length 2".into()); }
                let code = arr[0].as_i64().ok_or_else(|| "dmp op code must be number".to_string())?;
                let s = arr[1].as_str().ok_or_else(|| "dmp op text must be string".to_string())?;
                (code, s.to_string())
            } else if let Some(obj) = item.as_object() {
                let code = obj.get("op").and_then(|v| v.as_i64()).ok_or_else(|| "dmp op 'op' must be number".to_string())?;
                let s = obj.get("text").and_then(|v| v.as_str()).ok_or_else(|| "dmp op 'text' must be string".to_string())?;
                (code, s.to_string())
            } else {
                return Err("Invalid dmp op item".into());
            };

            match op_code {
                0 => { // equal
                    if !src.starts_with(&seg) { return Err("dmp equal segment does not match source".into()); }
                    out.push_str(&seg);
                    src = &src[seg.len()..];
                }
                -1 => { // delete
                    if !src.starts_with(&seg) { return Err("dmp delete segment does not match source".into()); }
                    src = &src[seg.len()..];
                }
                1 => { // insert
                    out.push_str(&seg);
                }
                _ => return Err("Unknown dmp op code".into()),
            }
        }
        // Append any remaining source (if DMP didn't cover tail)
        out.push_str(src);
        Ok(out)
    }
}

impl AgentTool for PatchFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "patch_file".to_string(),
                description: "Patch a file with minimal edits using structured operations. Prefer this over overwrite for updates.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "patch": {
                            "oneOf": [
                                { 
                                    "type": "object", 
                                    "properties": { 
                                        "ops": { 
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "oneOf": [
                                                    {
                                                        "type": "object",
                                                        "properties": {
                                                            "type": { "type": "string", "const": "replaceRange" },
                                                            "start_line": { "type": "number" },
                                                            "start_col": { "type": "number" },
                                                            "end_line": { "type": "number" },
                                                            "end_col": { "type": "number" },
                                                            "text": { "type": "string" }
                                                        },
                                                        "required": ["type", "start_line", "start_col", "end_line", "end_col", "text"]
                                                    },
                                                    {
                                                        "type": "object",
                                                        "properties": {
                                                            "type": { "type": "string", "const": "insertAt" },
                                                            "line": { "type": "number" },
                                                            "col": { "type": "number" },
                                                            "text": { "type": "string" }
                                                        },
                                                        "required": ["type", "line", "col", "text"]
                                                    },
                                                    {
                                                        "type": "object",
                                                        "properties": {
                                                            "type": { "type": "string", "const": "deleteRange" },
                                                            "start_line": { "type": "number" },
                                                            "start_col": { "type": "number" },
                                                            "end_line": { "type": "number" },
                                                            "end_col": { "type": "number" }
                                                        },
                                                        "required": ["type", "start_line", "start_col", "end_line", "end_col"]
                                                    }
                                                ]
                                            }
                                        }
                                    }, 
                                    "required": ["ops"] 
                                },
                                { "type": "object", "properties": { "unified_diff": { "type": "string" } }, "required": ["unified_diff"] },
                                { 
                                    "type": "object", 
                                    "properties": { 
                                        "dmp_ops": { 
                                            "type": "array", 
                                            "items": { 
                                                "type": "object",
                                                "properties": {
                                                    "op": { "type": "number" },
                                                    "text": { "type": "string" }
                                                },
                                                "required": ["op", "text"]
                                            } 
                                        } 
                                    }, 
                                    "required": ["dmp_ops"] 
                                }
                            ]
                        },
                        "precondition": {
                            "type": "object",
                            "properties": { "mtime": { "type": "number" }, "on_mismatch": { "type": "string", "enum": ["fail", "rebase"] } }
                        }
                    },
                    "required": ["path", "patch"]
                }),
                display_name: Some("Patch File".to_string()),
                progressive_form: Some("Patching file".to_string()),
            },
        }
    }

    async fn execute(&self, workspace_path: &PathBuf, args: Value, _app_handle: Option<&tauri::AppHandle>) -> AgentResult<String> {
        let path = args["path"].as_str().ok_or_else(|| AgentError::InvalidToolArguments { tool: "patch_file".into(), error: "Missing 'path'".into() })?;
        let full_path = validate_workspace_path(workspace_path, path)?;

        // Load current content
        let mut current = String::new();
        let existed = full_path.exists();
        if existed {
            current = fs::read_to_string(&full_path).await.map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: format!("Failed to read file: {}", e) })?;
        }

        // Precondition: mtime
        if let Some(pre) = args.get("precondition") {
            if let Some(mtime) = pre.get("mtime").and_then(|v| v.as_u64()) {
                if let Ok(meta) = tokio::fs::metadata(&full_path).await {
                    if let Ok(modified) = meta.modified() {
                        let ts = modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                        if ts != mtime {
                            let on = pre.get("on_mismatch").and_then(|v| v.as_str()).unwrap_or("fail");
                            if on == "rebase" {
                                return Err(AgentError::ToolExecutionError { tool: "patch_file".into(), error: "Precondition mismatch; rebase not implemented".into() });
                            }
                            return Err(AgentError::ToolExecutionError { tool: "patch_file".into(), error: "Precondition mismatch (mtime)".into() });
                        }
                    }
                }
            }
        }

        // Parse patch
        let patch_payload: PatchPayload = serde_json::from_value(args.get("patch").cloned().ok_or_else(|| AgentError::InvalidToolArguments { tool: "patch_file".into(), error: "Missing 'patch'".into() })?)
            .map_err(|e| AgentError::InvalidToolArguments { tool: "patch_file".into(), error: format!("Invalid patch payload: {}", e) })?;

        let updated = match patch_payload {
            PatchPayload::Ops { ops } => Self::apply_ops(&current, &ops)
                .map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: e })?,
            PatchPayload::UnifiedDiff { unified_diff } => Self::apply_unified_diff(&current, &unified_diff)
                .map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: e })?,
            PatchPayload::DmpOps { dmp_ops } => Self::apply_dmp_ops(&current, &dmp_ops)
                .map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: e })?,
        };

        // Ensure parent exists
        if let Some(parent) = full_path.parent() { if !parent.exists() { fs::create_dir_all(parent).await.map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: format!("Failed to create parent: {}", e) })?; } }

        // Atomic write: tempfile then rename
        let tmp_path = full_path.with_extension(".patching.tmp");
        {
            let mut f = fs::File::create(&tmp_path).await.map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: format!("Failed to create temp file: {}", e) })?;
            f.write_all(updated.as_bytes()).await.map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: format!("Failed to write temp file: {}", e) })?;
            f.flush().await.ok();
        }
        fs::rename(&tmp_path, &full_path).await.map_err(|e| AgentError::ToolExecutionError { tool: "patch_file".into(), error: format!("Failed to move temp file: {}", e) })?;

        let lines = updated.lines().count();
        let bytes = updated.len();
        let action = if existed { "patched" } else { "created" };
        Ok(format!("Successfully {} file '{}' ({} bytes, {} lines)", action, path, bytes, lines))
    }
}
