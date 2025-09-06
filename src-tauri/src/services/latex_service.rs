use crate::models::{LaTeXCompileRequest, LaTeXCompileResponse, LaTeXProvider};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

pub struct LaTeXService {
    workspace_path: PathBuf,
    compilation_lock: Arc<Mutex<()>>,
}

impl LaTeXService {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            workspace_path,
            compilation_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn compile_workspace(&self, request: LaTeXCompileRequest) -> Result<LaTeXCompileResponse, String> {
        // Acquire lock to prevent concurrent compilations
        let _lock = match self.compilation_lock.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                return Ok(LaTeXCompileResponse::error(
                    "Another compilation is already in progress".to_string(),
                    vec!["Please wait for the current compilation to finish".to_string()],
                ));
            }
        };

        info!("Starting LaTeX compilation with provider: {:?}", request.provider);

        // Check if latexmk is available
        if !self.check_latexmk_available().await {
            return Ok(LaTeXCompileResponse::error(
                "latexmk is not available".to_string(),
                vec!["Please install latexmk to compile LaTeX documents".to_string()],
            ));
        }

        // Find main tex file
        let main_tex_file = match self.find_main_tex_file().await {
            Ok(file) => file,
            Err(e) => {
                return Ok(LaTeXCompileResponse::error(
                    "Could not find main LaTeX file".to_string(),
                    vec![e.to_string()],
                ));
            }
        };

        // Determine which engines to try
        let engines_to_try = match request.provider {
            LaTeXProvider::Auto => {
                self.detect_available_engines().await
            }
            _ => {
                if let Some(engine) = request.provider.engine_name() {
                    if self.check_engine_available(engine).await {
                        vec![engine.to_string()]
                    } else {
                        return Ok(LaTeXCompileResponse::error(
                            format!("Specified engine '{}' is not available", engine),
                            vec![format!("Please install {} or use auto detection", engine)],
                        ));
                    }
                } else {
                    self.detect_available_engines().await
                }
            }
        };

        if engines_to_try.is_empty() {
            return Ok(LaTeXCompileResponse::error(
                "No LaTeX engines available".to_string(),
                vec!["Please install pdflatex, xelatex, or lualatex".to_string()],
            ));
        }

        // Try each engine until one succeeds
        let mut last_errors = Vec::new();
        
        for engine in &engines_to_try {
            info!("Attempting compilation with engine: {}", engine);
            
            match self.execute_latexmk(&main_tex_file, engine).await {
                Ok((_stdout, _stderr)) => {
                    // Compilation succeeded
                    let pdf_path = self.extract_pdf_path(&main_tex_file);
                    let relative_pdf_path = pdf_path.strip_prefix(&self.workspace_path)
                        .unwrap_or(&pdf_path)
                        .to_string_lossy()
                        .to_string();

                    info!("LaTeX compilation successful with engine: {}", engine);
                    
                    return Ok(LaTeXCompileResponse::success(
                        format!("Compilation successful with {}", engine),
                        Some(relative_pdf_path),
                    ));
                }
                Err(e) => {
                    warn!("Compilation failed with engine {}: {}", engine, e);
                    
                    // Extract errors from the compilation output
                    let error_output = e.to_string();
                    let parsed_errors = Self::parse_latex_errors(&error_output, "");
                    last_errors = parsed_errors;
                    
                    // If this was a user-specified engine (not auto), don't try others
                    if request.provider.engine_name().is_some() {
                        break;
                    }
                }
            }
        }

        // All engines failed
        let engines_tried = engines_to_try.join(", ");
        let error_message = if engines_to_try.len() == 1 {
            format!("Compilation failed with engine: {}", engines_tried)
        } else {
            format!("Compilation failed with all available engines: {}", engines_tried)
        };

        error!("LaTeX compilation failed after trying engines: {}", engines_tried);
        
        Ok(LaTeXCompileResponse::error(
            error_message,
            if last_errors.is_empty() {
                vec!["Unknown compilation error occurred".to_string()]
            } else {
                last_errors
            },
        ))
    }

    async fn find_main_tex_file(&self) -> Result<PathBuf, String> {
        let mut tex_files = Vec::new();
        
        // Find all .tex files in the workspace
        self.collect_tex_files(&self.workspace_path, &mut tex_files).await
            .map_err(|e| format!("Failed to scan for tex files: {}", e))?;
        
        if tex_files.is_empty() {
            return Err("No .tex files found in workspace".to_string());
        }

        // Look for main document file (contains \documentclass)
        for tex_file in &tex_files {
            if self.is_main_tex_file(tex_file).await.unwrap_or(false) {
                info!("Found main LaTeX file: {:?}", tex_file);
                return Ok(tex_file.clone());
            }
        }

        // If no main file found, use the first .tex file
        let main_file = tex_files.into_iter().next().unwrap();
        warn!("No main LaTeX file with \\documentclass found, using first .tex file: {:?}", main_file);
        Ok(main_file)
    }

    fn collect_tex_files<'a>(&'a self, dir: &'a Path, tex_files: &'a mut Vec<PathBuf>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), std::io::Error>> + Send + 'a>> {
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
                        self.collect_tex_files(&path, tex_files).await?;
                    }
                } else if path.extension().and_then(|s| s.to_str()) == Some("tex") {
                    tex_files.push(path);
                }
            }
            
            Ok(())
        })
    }

    async fn is_main_tex_file(&self, file_path: &Path) -> Result<bool, std::io::Error> {
        let content = tokio::fs::read_to_string(file_path).await?;
        
        // Look for \documentclass command which indicates a main document
        Ok(content.contains("\\documentclass"))
    }

    async fn execute_latexmk(&self, tex_file: &Path, engine: &str) -> Result<(String, String), String> {
        let tex_file_dir = tex_file.parent()
            .ok_or_else(|| "Cannot get parent directory of tex file".to_string())?;
        
        let tex_filename = tex_file.file_name()
            .ok_or_else(|| "Cannot get filename of tex file".to_string())?;

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
            _ => return Err(format!("Unsupported LaTeX engine: {}", engine)),
        }

        cmd.arg(tex_filename);

        let output = cmd.output().await
            .map_err(|e| format!("Failed to execute latexmk: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        debug!("latexmk stdout: {}", stdout);
        debug!("latexmk stderr: {}", stderr);

        if !output.status.success() {
            return Err(format!("LaTeX compilation failed with engine {}: {}", engine, stderr));
        }

        Ok((stdout, stderr))
    }

    async fn check_engine_available(&self, engine: &str) -> bool {
        let output = Command::new(engine)
            .arg("--version")
            .output()
            .await;
        
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    async fn check_latexmk_available(&self) -> bool {
        let output = Command::new("latexmk")
            .arg("-version")
            .output()
            .await;
        
        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    fn extract_pdf_path(&self, tex_file: &Path) -> PathBuf {
        let mut pdf_path = tex_file.to_path_buf();
        pdf_path.set_extension("pdf");
        pdf_path
    }

    fn parse_latex_errors(stderr: &str, stdout: &str) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Combine stderr and stdout for error parsing
        let combined_output = format!("{}\n{}", stderr, stdout);
        
        // Simple error patterns for LaTeX compilation errors
        let lines: Vec<&str> = combined_output.lines().collect();
        
        for line in lines {
            let line = line.trim();
            
            // Look for common LaTeX error patterns
            if line.starts_with("!") || 
               line.contains("Error:") || 
               line.contains("error:") ||
               (line.contains(":") && line.contains("undefined")) {
                if !errors.contains(&line.to_string()) {
                    errors.push(line.to_string());
                }
            }
        }
        
        // If no structured errors found, include key parts of stderr
        if errors.is_empty() && !stderr.is_empty() {
            errors.push(stderr.lines().take(5).collect::<Vec<_>>().join("\n"));
        }
        
        errors
    }

    async fn detect_available_engines(&self) -> Vec<String> {
        let mut available_engines = Vec::new();
        
        // Check engines in priority order
        for engine in LaTeXProvider::get_auto_priority_engines() {
            if self.check_engine_available(engine).await {
                debug!("Engine {} is available", engine);
                available_engines.push(engine.to_string());
            } else {
                debug!("Engine {} is not available", engine);
            }
        }
        
        if available_engines.is_empty() {
            warn!("No LaTeX engines detected on the system");
        } else {
            info!("Available LaTeX engines: {:?}", available_engines);
        }
        
        available_engines
    }
}