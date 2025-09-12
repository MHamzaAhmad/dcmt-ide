use anyhow::{anyhow, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct EngineAttempt {
    pub engine: String,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub error_message: Option<String>,
}

pub struct LaTeXRepository {
    pub workspace_path: PathBuf,
}

impl LaTeXRepository {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    pub async fn find_main_tex_file(&self) -> Result<PathBuf> {
        let mut tex_files = Vec::new();
        
        // Find all .tex files in the workspace
        Self::collect_tex_files(&self.workspace_path, &mut tex_files).await?;
        
        if tex_files.is_empty() {
            return Err(anyhow!("No .tex files found in workspace"));
        }

        // Look for main document file (contains \documentclass)
        for tex_file in &tex_files {
            if self.is_main_tex_file(tex_file).await? {
                info!("Found main LaTeX file: {:?}", tex_file);
                return Ok(tex_file.clone());
            }
        }

        // If no main file found, use the first .tex file
        let main_file = tex_files.into_iter().next().unwrap();
        warn!("No main LaTeX file with \\documentclass found, using first .tex file: {:?}", main_file);
        Ok(main_file)
    }

    fn collect_tex_files<'a>(dir: &'a Path, tex_files: &'a mut Vec<PathBuf>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut entries = tokio::fs::read_dir(dir).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip common build/output directories
                    let dir_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if !matches!(dir_name, "build" | "dist" | "output" | ".git" | "node_modules" | "target") {
                        Self::collect_tex_files(&path, tex_files).await?;
                    }
                } else if path.extension().and_then(|s| s.to_str()) == Some("tex") {
                    tex_files.push(path);
                }
            }
            
            Ok(())
        })
    }

    async fn is_main_tex_file(&self, file_path: &Path) -> Result<bool> {
        let content = tokio::fs::read_to_string(file_path).await?;
        
        // Look for \documentclass command which indicates a main document
        let documentclass_regex = Regex::new(r"\\documentclass")?;
        
        Ok(documentclass_regex.is_match(&content))
    }

    pub async fn execute_latexmk(&self, tex_file: &Path, engine: &str) -> Result<EngineAttempt> {
        let tex_file_dir = tex_file.parent()
            .ok_or_else(|| anyhow!("Cannot get parent directory of tex file"))?;
        
        let tex_filename = tex_file.file_name()
            .ok_or_else(|| anyhow!("Cannot get filename of tex file"))?;

        debug!("Executing latexmk with engine: {} for file: {:?}", engine, tex_file);

        let mut cmd = Command::new("latexmk");
        cmd.current_dir(tex_file_dir)
            .arg("-interaction=nonstopmode")
            .arg("-file-line-error")
            .arg("-halt-on-error")
            .arg("-synctex=1");

        // Set engine-specific options
        match engine {
            "pdflatex" => {
                cmd.arg("-pdf");
            }
            "xelatex" => {
                cmd.arg("-xelatex");
            }
            "lualatex" => {
                cmd.arg("-lualatex");
            }
            _ => return Err(anyhow!("Unsupported LaTeX engine: {}", engine)),
        }

        cmd.arg(tex_filename);

        let output = cmd.output().await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        debug!("latexmk stdout: {}", stdout);
        debug!("latexmk stderr: {}", stderr);

        let success = output.status.success();
        let error_message = if !success {
            Some(format!("LaTeX compilation failed with engine {}", engine))
        } else {
            None
        };

        Ok(EngineAttempt {
            engine: engine.to_string(),
            stdout,
            stderr,
            success,
            error_message,
        })
    }

    pub async fn check_engine_available(&self, engine: &str) -> bool {
        let output = Command::new(engine)
            .arg("--version")
            .output()
            .await;
        
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub async fn check_latexmk_available(&self) -> bool {
        let output = Command::new("latexmk")
            .arg("-version")
            .output()
            .await;
        
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub fn extract_pdf_path(&self, tex_file: &Path) -> PathBuf {
        let mut pdf_path = tex_file.to_path_buf();
        pdf_path.set_extension("pdf");
        pdf_path
    }

    pub fn get_raw_latex_output(stderr: &str, stdout: &str) -> Vec<String> {
        // Return raw latexmk output as-is for human and agent readability
        // LaTeX compilation output is already detailed and informative
        let mut output_lines = Vec::new();
        
        // Include both stdout and stderr as separate sections if they contain content
        if !stdout.trim().is_empty() {
            output_lines.push(format!("=== LaTeX Compilation Output ===\n{}", stdout.trim()));
        }
        
        if !stderr.trim().is_empty() {
            output_lines.push(format!("=== LaTeX Error Output ===\n{}", stderr.trim()));
        }
        
        // If both are empty, provide a generic message
        if output_lines.is_empty() {
            output_lines.push("LaTeX compilation failed with no output".to_string());
        }
        
        output_lines
    }

    pub fn get_multi_engine_output(engine_attempts: &[EngineAttempt]) -> Vec<String> {
        let mut output_lines = Vec::new();
        
        // Process each engine attempt
        for attempt in engine_attempts {
            output_lines.push(format!("=== Engine: {} ===", attempt.engine));
            
            if !attempt.stdout.trim().is_empty() {
                output_lines.push(format!("Compilation Output:\n{}", attempt.stdout.trim()));
            }
            
            if !attempt.stderr.trim().is_empty() {
                output_lines.push(format!("Error Output:\n{}", attempt.stderr.trim()));
            }
            
            if let Some(ref error_msg) = attempt.error_message {
                output_lines.push(format!("Result: {}", error_msg));
            } else {
                output_lines.push("Result: Success".to_string());
            }
            
            output_lines.push("".to_string()); // Empty line between engines
        }
        
        // Add summary
        if engine_attempts.len() > 1 {
            let failed_engines: Vec<String> = engine_attempts.iter()
                .filter(|attempt| !attempt.success)
                .map(|attempt| attempt.engine.clone())
                .collect();
                
            if !failed_engines.is_empty() {
                output_lines.push(format!("=== Summary ==="));
                if failed_engines.len() == engine_attempts.len() {
                    output_lines.push(format!("All engines failed: {}", failed_engines.join(", ")));
                } else {
                    output_lines.push(format!("Failed engines: {}", failed_engines.join(", ")));
                }
            }
        }
        
        // If no output at all, provide a generic message
        if output_lines.is_empty() {
            output_lines.push("LaTeX compilation failed with no output".to_string());
        }
        
        output_lines
    }
}