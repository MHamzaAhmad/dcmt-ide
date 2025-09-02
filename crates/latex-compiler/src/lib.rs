use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::fs;
use tracing::{debug, info};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Engine {
    PdfLatex,
    XeLatex,
    LuaLatex,
}

impl Engine {
    pub fn command(&self) -> &str {
        match self {
            Engine::PdfLatex => "pdflatex",
            Engine::XeLatex => "xelatex",
            Engine::LuaLatex => "lualatex",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationOptions {
    pub engine: Engine,
    pub output_dir: Option<PathBuf>,
    pub aux_dir: Option<PathBuf>,
    pub shell_escape: bool,
    pub interaction_mode: InteractionMode,
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {
            engine: Engine::PdfLatex,
            output_dir: None,
            aux_dir: None,
            shell_escape: false,
            interaction_mode: InteractionMode::NonStop,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InteractionMode {
    BatchMode,
    NonStop,
    ScrollMode,
    ErrorStop,
}

impl InteractionMode {
    pub fn flag(&self) -> &str {
        match self {
            InteractionMode::BatchMode => "-interaction=batchmode",
            InteractionMode::NonStop => "-interaction=nonstopmode",
            InteractionMode::ScrollMode => "-interaction=scrollmode",
            InteractionMode::ErrorStop => "-interaction=errorstopmode",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    pub success: bool,
    pub pdf_path: Option<PathBuf>,
    pub log: String,
    pub errors: Vec<CompilationError>,
    pub warnings: Vec<CompilationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationError {
    pub line: Option<usize>,
    pub message: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationWarning {
    pub line: Option<usize>,
    pub message: String,
    pub file: Option<String>,
}

pub struct LaTeXCompiler {
    options: CompilationOptions,
    working_dir: PathBuf,
    #[cfg(feature = "git-integration")]
    session_manager: Option<latex_ide_git_manager::SessionManager>,
}

impl LaTeXCompiler {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            options: CompilationOptions::default(),
            working_dir,
            #[cfg(feature = "git-integration")]
            session_manager: None,
        }
    }

    #[cfg(feature = "git-integration")]
    pub fn with_git_integration(mut self, session_manager: latex_ide_git_manager::SessionManager) -> Self {
        self.session_manager = Some(session_manager);
        self
    }
    
    pub fn with_options(mut self, options: CompilationOptions) -> Self {
        self.options = options;
        self
    }
    
    pub async fn compile(&self, tex_file: &Path) -> Result<CompilationResult> {
        info!("Starting LaTeX compilation for: {:?}", tex_file);
        
        // Ensure the tex file exists
        if !tex_file.exists() {
            return Err(anyhow::anyhow!("TeX file does not exist: {:?}", tex_file));
        }
        
        // Build command
        let mut cmd = Command::new(self.options.engine.command());
        
        // Add flags
        cmd.arg(self.options.interaction_mode.flag());
        
        if self.options.shell_escape {
            cmd.arg("-shell-escape");
        }
        
        if let Some(output_dir) = &self.options.output_dir {
            cmd.arg(format!("-output-directory={}", output_dir.display()));
        }
        
        if let Some(aux_dir) = &self.options.aux_dir {
            cmd.arg(format!("-aux-directory={}", aux_dir.display()));
        }
        
        // Add the tex file
        cmd.arg(tex_file);
        
        // Set working directory
        cmd.current_dir(&self.working_dir);
        
        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        debug!("Executing command: {:?}", cmd);
        
        // Run compilation
        let output = cmd.output().await?;
        
        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let log = format!("{}\n{}", stdout, stderr);
        
        // Parse errors and warnings
        let (errors, warnings) = self.parse_log(&log);
        
        // Determine PDF path
        let pdf_path = if output.status.success() {
            let stem = tex_file.file_stem().unwrap().to_string_lossy();
            let pdf_name = format!("{}.pdf", stem);
            
            let pdf_path = if let Some(output_dir) = &self.options.output_dir {
                output_dir.join(pdf_name)
            } else {
                self.working_dir.join(pdf_name)
            };
            
            if pdf_path.exists() {
                Some(pdf_path)
            } else {
                None
            }
        } else {
            None
        };
        
        let result = CompilationResult {
            success: output.status.success(),
            pdf_path: pdf_path.clone(),
            log,
            errors,
            warnings,
        };

        // Auto-commit on successful compilation if Git integration is enabled
        #[cfg(feature = "git-integration")]
        if result.success && result.pdf_path.is_some() {
            if let Some(ref session_manager) = self.session_manager {
                let tex_file_str = tex_file.to_string_lossy();
                let pdf_path_str = result.pdf_path.as_ref().unwrap().to_string_lossy();
                
                match session_manager.commit_pdf_version(&pdf_path_str, &tex_file_str) {
                    Ok((commit_id, version)) => {
                        info!("Auto-committed PDF compilation: {} ({})", commit_id, version);
                    }
                    Err(e) => {
                        debug!("Failed to auto-commit PDF: {}", e);
                    }
                }
            }
        }

        Ok(result)
    }
    
    pub async fn compile_string(&self, content: &str) -> Result<CompilationResult> {
        // Create a temporary file
        let temp_dir = tempfile::tempdir()?;
        let tex_file = temp_dir.path().join("document.tex");
        
        // Write content to file
        fs::write(&tex_file, content).await?;
        
        // Compile
        let result = self.compile(&tex_file).await?;
        
        // If successful, copy PDF to working directory
        if let Some(pdf_path) = &result.pdf_path {
            let dest = self.working_dir.join("output.pdf");
            fs::copy(pdf_path, &dest).await?;
            
            return Ok(CompilationResult {
                pdf_path: Some(dest),
                ..result
            });
        }
        
        Ok(result)
    }
    
    fn parse_log(&self, log: &str) -> (Vec<CompilationError>, Vec<CompilationWarning>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        let error_regex = regex::Regex::new(r"! (.+)").unwrap();
        let line_regex = regex::Regex::new(r"l\.(\d+)").unwrap();
        let warning_regex = regex::Regex::new(r"Warning: (.+)").unwrap();
        
        let lines: Vec<&str> = log.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            // Check for errors
            if let Some(caps) = error_regex.captures(line) {
                let message = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                
                // Try to find line number in next few lines
                let mut line_num = None;
                for j in 1..=3 {
                    if i + j < lines.len() {
                        if let Some(line_caps) = line_regex.captures(lines[i + j]) {
                            line_num = line_caps.get(1)
                                .and_then(|m| m.as_str().parse::<usize>().ok());
                            break;
                        }
                    }
                }
                
                errors.push(CompilationError {
                    line: line_num,
                    message,
                    file: None,
                });
            }
            
            // Check for warnings
            if let Some(caps) = warning_regex.captures(line) {
                let message = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                warnings.push(CompilationWarning {
                    line: None,
                    message,
                    file: None,
                });
            }
        }
        
        (errors, warnings)
    }
    
    pub async fn check_engine_available(&self) -> bool {
        Command::new(self.options.engine.command())
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_compiler_creation() {
        let compiler = LaTeXCompiler::new(PathBuf::from("/tmp"));
        assert!(compiler.working_dir == PathBuf::from("/tmp"));
    }
}