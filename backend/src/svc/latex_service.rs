use crate::model::{LaTeXCompileRequest, LaTeXCompileResponse, LaTeXProvider};
use crate::repo::LaTeXRepository;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct LaTeXService {
    repository: Arc<LaTeXRepository>,
    compilation_lock: Arc<Mutex<()>>,
}

impl LaTeXService {
    pub fn new(workspace_path: PathBuf) -> Self {
        let repository = Arc::new(LaTeXRepository::new(workspace_path));
        let compilation_lock = Arc::new(Mutex::new(()));

        Self {
            repository,
            compilation_lock,
        }
    }

    pub async fn compile_workspace(&self, request: LaTeXCompileRequest) -> Result<LaTeXCompileResponse> {
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
        if !self.repository.check_latexmk_available().await {
            return Ok(LaTeXCompileResponse::error(
                "latexmk is not available".to_string(),
                vec!["Please install latexmk to compile LaTeX documents".to_string()],
            ));
        }

        // Find main tex file
        let main_tex_file = match self.repository.find_main_tex_file().await {
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
                    if self.repository.check_engine_available(engine).await {
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
            
            match self.repository.execute_latexmk(&main_tex_file, engine).await {
                Ok((_stdout, _stderr)) => {
                    // Compilation succeeded
                    let pdf_path = self.repository.extract_pdf_path(&main_tex_file);
                    let relative_pdf_path = pdf_path.strip_prefix(&self.repository.workspace_path)
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
                    let parsed_errors = LaTeXRepository::parse_latex_errors(&error_output, "");
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

    async fn detect_available_engines(&self) -> Vec<String> {
        let mut available_engines = Vec::new();
        
        // Check engines in priority order
        for engine in LaTeXProvider::get_auto_priority_engines() {
            if self.repository.check_engine_available(engine).await {
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

    pub async fn find_main_tex_file(&self) -> Result<PathBuf> {
        self.repository.find_main_tex_file().await
    }

    pub fn get_workspace_path(&self) -> &PathBuf {
        &self.repository.workspace_path
    }
}