use anyhow::{anyhow, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

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

    pub async fn execute_latexmk(&self, tex_file: &Path, engine: &str) -> Result<(String, String)> {
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

        if !output.status.success() {
            return Err(anyhow!("LaTeX compilation failed with engine {}: {}", engine, stderr));
        }

        Ok((stdout, stderr))
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

    pub fn parse_latex_errors(stderr: &str, stdout: &str) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Combine stderr and stdout for error parsing
        let combined_output = format!("{}\n{}", stderr, stdout);
        
        // Regex patterns for common LaTeX errors
        let error_patterns = vec![
            Regex::new(r"(?m)^.*?:(\d+):\s*(.+?)$").unwrap(),
            Regex::new(r"(?m)^!\s*(.+?)$").unwrap(),
            Regex::new(r"(?m)^.*?Error:\s*(.+?)$").unwrap(),
        ];
        
        for pattern in error_patterns {
            for cap in pattern.captures_iter(&combined_output) {
                if cap.len() > 1 {
                    let error_msg = if cap.len() > 2 {
                        format!("Line {}: {}", &cap[1], &cap[2].trim())
                    } else {
                        cap[1].trim().to_string()
                    };
                    
                    if !error_msg.is_empty() && !errors.contains(&error_msg) {
                        errors.push(error_msg);
                    }
                }
            }
        }
        
        // If no structured errors found, include key parts of stderr
        if errors.is_empty() && !stderr.is_empty() {
            errors.push(stderr.lines().take(5).collect::<Vec<_>>().join("\n"));
        }
        
        errors
    }
}