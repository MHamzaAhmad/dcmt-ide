use crate::models::{LaTeXCompileRequest, LaTeXCompileResponse, LaTeXProvider, CompilationEvent};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use tokio::process::Command;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use tauri::{AppHandle, Emitter, Listener};

#[derive(Clone)]
pub struct LaTeXService {
    workspace_path: PathBuf,
    compilation_lock: Arc<Mutex<()>>,
    app_handle: AppHandle,
    main_tex_file: Arc<RwLock<Option<PathBuf>>>,
    debounce_timers: Arc<RwLock<HashMap<String, Instant>>>,
    is_auto_compile_enabled: Arc<RwLock<bool>>,
}

impl LaTeXService {
    pub fn new(workspace_path: PathBuf, app_handle: AppHandle) -> Self {
        let service = Self {
            workspace_path,
            compilation_lock: Arc::new(Mutex::new(())),
            app_handle: app_handle.clone(),
            main_tex_file: Arc::new(RwLock::new(None)),
            debounce_timers: Arc::new(RwLock::new(HashMap::new())),
            is_auto_compile_enabled: Arc::new(RwLock::new(true)),
        };
        
        // Set up file change event listener
        service.setup_file_change_listener();
        
        service
    }
    
    /// Set up listener for file change events from the file watcher
    fn setup_file_change_listener(&self) {
        let service_clone = self.clone();
        let app_handle = self.app_handle.clone();
        
        // Listen to file modification events
        let _listener = app_handle.listen("file-modified", move |event| {
            if let Ok(file_event) = serde_json::from_str::<crate::models::FileEvent>(
                &event.payload().to_string()
            ) {
                let service_clone = service_clone.clone();
                tokio::spawn(async move {
                    service_clone.handle_file_change(&file_event.path).await;
                });
            }
        });
        
        info!("LaTeX service file change listener set up");
    }

    pub async fn set_auto_compile(&self, enabled: bool) {
        *self.is_auto_compile_enabled.write().await = enabled;
        info!("Auto-compilation {}", if enabled { "enabled" } else { "disabled" });
    }

    pub async fn get_auto_compile(&self) -> bool {
        *self.is_auto_compile_enabled.read().await
    }

    /// Smart file filtering - determines if a file change should trigger compilation
    pub fn should_trigger_compilation(&self, file_path: &str) -> bool {
        let path = file_path.to_lowercase();
        
        // Include LaTeX ecosystem files
        let include_extensions = [".tex", ".bib", ".sty", ".cls", ".def", ".cfg", ".clo"];
        let should_include = include_extensions.iter().any(|ext| path.ends_with(ext));
        
        if !should_include {
            return false;
        }
        
        // Exclude auxiliary and generated files
        let exclude_patterns = [
            ".aux", ".log", ".out", ".fdb_latexmk", ".fls", ".synctex.gz",
            ".toc", ".lof", ".lot", ".bbl", ".blg", ".idx", ".ind", ".ilg",
            ".nav", ".snm", ".vrb", ".fls", ".figlist", ".makefile", ".figdir",
            ".figdist", ".figpdf", ".run.xml", ".bcf", ".glg", ".glo", ".gls",
            ".ist", ".xdy", ".acn", ".acr", ".alg", ".glsdefs", ".lol",
            "texput.log", ".auxlock", ".tmp", ".temp", ".backup", ".bak",
            ".orig", ".rej", ".dpth", ".md5", ".auxdata"
        ];
        
        // Don't include if it matches exclude patterns
        let should_exclude = exclude_patterns.iter().any(|pattern| path.contains(pattern));
        
        if should_exclude {
            debug!("Excluding file from compilation trigger: {}", file_path);
            return false;
        }
        
        // Don't include PDF files (outputs, not inputs)
        if path.ends_with(".pdf") {
            return false;
        }
        
        debug!("File change will trigger compilation: {}", file_path);
        true
    }

    /// Handle file change events with debouncing
    pub async fn handle_file_change(&self, file_path: &str) {
        if !self.should_trigger_compilation(file_path) {
            return;
        }

        if !*self.is_auto_compile_enabled.read().await {
            debug!("Auto-compilation disabled, ignoring file change: {}", file_path);
            return;
        }

        let main_file = {
            let main_file_guard = self.main_tex_file.read().await;
            if main_file_guard.is_none() {
                // Try to detect main file if not set
                drop(main_file_guard);
                if let Err(e) = self.detect_and_set_main_file().await {
                    warn!("Could not detect main LaTeX file: {}", e);
                    return;
                }
                self.main_tex_file.read().await.clone()
            } else {
                main_file_guard.clone()
            }
        };

        if let Some(main_file) = main_file {
            let main_file_str = main_file.to_string_lossy().to_string();
            
            // Emit queued event
            let queued_event = CompilationEvent::queued(
                main_file_str.clone(),
                format!("file_change:{}", file_path)
            );
            self.emit_compilation_event(queued_event).await;

            // Start debounced compilation
            self.schedule_debounced_compilation(main_file_str, "file_change").await;
        }
    }

    /// Schedule compilation with debouncing (1000ms delay)
    async fn schedule_debounced_compilation(&self, main_file: String, reason: &str) {
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(1000);
        
        let timer_key = main_file.clone();
        let now = Instant::now();
        
        // Update the debounce timer
        {
            let mut timers = self.debounce_timers.write().await;
            timers.insert(timer_key.clone(), now);
        }
        
        // Clone necessary data for the async task
        let service = self.clone();
        let reason = reason.to_string();
        
        tokio::spawn(async move {
            tokio::time::sleep(DEBOUNCE_DURATION).await;
            
            // Check if this timer is still the latest
            let should_compile = {
                let timers = service.debounce_timers.read().await;
                timers.get(&timer_key).map_or(false, |&timer_time| timer_time == now)
            };
            
            if should_compile {
                info!("Debounced compilation triggered for: {} (reason: {})", main_file, reason);
                
                // Clear the timer
                {
                    let mut timers = service.debounce_timers.write().await;
                    timers.remove(&timer_key);
                }
                
                // Execute compilation
                let request = LaTeXCompileRequest {
                    provider: LaTeXProvider::Auto,
                };
                
                if let Err(e) = service.compile_workspace_internal(request, Some(reason)).await {
                    error!("Debounced compilation failed: {}", e);
                }
            } else {
                debug!("Debounced compilation cancelled (newer timer exists): {}", main_file);
            }
        });
    }

    /// Detect and set the main LaTeX file
    pub async fn detect_and_set_main_file(&self) -> Result<(), String> {
        match self.find_main_tex_file().await {
            Ok(main_file) => {
                let main_file_str = main_file.strip_prefix(&self.workspace_path)
                    .unwrap_or(&main_file)
                    .to_string_lossy()
                    .to_string();
                
                *self.main_tex_file.write().await = Some(main_file.clone());
                
                // Emit main file detected event
                let event = CompilationEvent::main_file_detected(main_file_str);
                self.emit_compilation_event(event).await;
                
                info!("Main LaTeX file detected: {:?}", main_file);
                Ok(())
            }
            Err(e) => {
                warn!("No main LaTeX file found: {}", e);
                Err(e)
            }
        }
    }

    /// Get the current main file
    pub async fn get_main_file(&self) -> Option<PathBuf> {
        self.main_tex_file.read().await.clone()
    }

    /// Manually set the main file
    pub async fn set_main_file(&self, main_file: PathBuf) {
        *self.main_tex_file.write().await = Some(main_file.clone());
        
        let main_file_str = main_file.strip_prefix(&self.workspace_path)
            .unwrap_or(&main_file)
            .to_string_lossy()
            .to_string();
        
        let event = CompilationEvent::main_file_detected(main_file_str);
        self.emit_compilation_event(event).await;
        
        info!("Main LaTeX file manually set: {:?}", main_file);
    }

    /// Emit compilation event via Tauri event system
    async fn emit_compilation_event(&self, event: CompilationEvent) {
        if let Err(e) = self.app_handle.emit("compilation-event", &event) {
            error!("Failed to emit compilation event: {}", e);
        }
    }

    pub async fn compile_workspace(&self, request: LaTeXCompileRequest) -> Result<LaTeXCompileResponse, String> {
        self.compile_workspace_internal(request, None).await
    }

    /// Internal compilation method with event emission
    async fn compile_workspace_internal(&self, request: LaTeXCompileRequest, _reason: Option<String>) -> Result<LaTeXCompileResponse, String> {
        // Acquire lock to prevent concurrent compilations
        let _lock = match self.compilation_lock.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                let error_response = LaTeXCompileResponse::error(
                    "Another compilation is already in progress".to_string(),
                    vec!["Please wait for the current compilation to finish".to_string()],
                );
                return Ok(error_response);
            }
        };

        let start_time = Instant::now();
        info!("Starting LaTeX compilation with provider: {:?}", request.provider);

        // Check if latexmk is available
        if !self.check_latexmk_available().await {
            let error_response = LaTeXCompileResponse::error(
                "latexmk is not available".to_string(),
                vec!["Please install latexmk to compile LaTeX documents".to_string()],
            );
            return Ok(error_response);
        }

        // Find main tex file
        let main_tex_file = match self.find_main_tex_file().await {
            Ok(file) => {
                // Update our cached main file
                *self.main_tex_file.write().await = Some(file.clone());
                file
            }
            Err(e) => {
                let error_response = LaTeXCompileResponse::error(
                    "Could not find main LaTeX file".to_string(),
                    vec![e.to_string()],
                );
                return Ok(error_response);
            }
        };

        let main_file_str = main_tex_file.strip_prefix(&self.workspace_path)
            .unwrap_or(&main_tex_file)
            .to_string_lossy()
            .to_string();

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
                        let error_event = CompilationEvent::error(
                            main_file_str.clone(),
                            vec![format!("Specified engine '{}' is not available", engine)],
                            Some(engine.to_string())
                        );
                        self.emit_compilation_event(error_event).await;

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
            let error_event = CompilationEvent::error(
                main_file_str.clone(),
                vec!["No LaTeX engines available".to_string()],
                None
            );
            self.emit_compilation_event(error_event).await;

            return Ok(LaTeXCompileResponse::error(
                "No LaTeX engines available".to_string(),
                vec!["Please install pdflatex, xelatex, or lualatex".to_string()],
            ));
        }

        // Try each engine until one succeeds
        let mut last_errors = Vec::new();
        
        for engine in &engines_to_try {
            info!("Attempting compilation with engine: {}", engine);
            
            // Emit started event
            let started_event = CompilationEvent::started(main_file_str.clone(), engine.clone());
            self.emit_compilation_event(started_event).await;
            
            match self.execute_latexmk(&main_tex_file, engine).await {
                Ok((_stdout, _stderr)) => {
                    // Compilation succeeded
                    let pdf_path = self.extract_pdf_path(&main_tex_file);
                    let relative_pdf_path = pdf_path.strip_prefix(&self.workspace_path)
                        .unwrap_or(&pdf_path)
                        .to_string_lossy()
                        .to_string();

                    let duration_ms = start_time.elapsed().as_millis() as u64;

                    info!("LaTeX compilation successful with engine: {}", engine);
                    
                    // Emit success event
                    let success_event = CompilationEvent::success(
                        main_file_str.clone(),
                        relative_pdf_path.clone(),
                        engine.clone(),
                        duration_ms
                    );
                    self.emit_compilation_event(success_event).await;
                    
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
        
        // Emit error event
        let error_event = CompilationEvent::error(
            main_file_str,
            if last_errors.is_empty() {
                vec!["Unknown compilation error occurred".to_string()]
            } else {
                last_errors.clone()
            },
            engines_to_try.first().cloned()
        );
        self.emit_compilation_event(error_event).await;
        
        Ok(LaTeXCompileResponse::error(
            error_message,
            if last_errors.is_empty() {
                vec!["Unknown compilation error occurred".to_string()]
            } else {
                last_errors
            },
        ))
    }

    /// Handle startup compilation
    pub async fn handle_startup_compilation(&self) -> Result<(), String> {
        info!("Handling startup compilation");
        
        // Detect main file first
        if let Err(e) = self.detect_and_set_main_file().await {
            warn!("Could not detect main LaTeX file on startup: {}", e);
            return Ok(()); // Don't fail startup if no main file
        }

        // Trigger compilation if main file exists
        if self.main_tex_file.read().await.is_some() {
            let request = LaTeXCompileRequest {
                provider: LaTeXProvider::Auto,
            };
            
            match self.compile_workspace_internal(request, Some("startup".to_string())).await {
                Ok(response) => {
                    if response.success {
                        info!("Startup compilation successful");
                    } else {
                        warn!("Startup compilation failed: {}", response.message);
                    }
                }
                Err(e) => {
                    error!("Startup compilation error: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Handle agent completion - trigger immediate compilation
    pub async fn handle_agent_completion(&self) -> Result<(), String> {
        info!("Handling agent completion compilation");
        
        if let Some(main_file) = self.main_tex_file.read().await.clone() {
            let main_file_str = main_file.to_string_lossy().to_string();
            
            // Clear any pending debounce timers for immediate compilation
            {
                let mut timers = self.debounce_timers.write().await;
                timers.clear();
            }
            
            // Emit queued event
            let queued_event = CompilationEvent::queued(
                main_file_str.clone(),
                "agent_completion".to_string()
            );
            self.emit_compilation_event(queued_event).await;

            // Execute compilation immediately (no debouncing)
            let request = LaTeXCompileRequest {
                provider: LaTeXProvider::Auto,
            };
            
            match self.compile_workspace_internal(request, Some("agent_completion".to_string())).await {
                Ok(response) => {
                    if response.success {
                        info!("Agent completion compilation successful");
                    } else {
                        warn!("Agent completion compilation failed: {}", response.message);
                    }
                }
                Err(e) => {
                    error!("Agent completion compilation error: {}", e);
                }
            }
        } else {
            debug!("No main LaTeX file available for agent completion compilation");
        }
        
        Ok(())
    }

    /// Force immediate compilation (manual trigger)
    pub async fn force_compile(&self) -> Result<LaTeXCompileResponse, String> {
        // Clear any pending debounce timers
        {
            let mut timers = self.debounce_timers.write().await;
            timers.clear();
        }
        
        let request = LaTeXCompileRequest {
            provider: LaTeXProvider::Auto,
        };
        
        self.compile_workspace_internal(request, Some("manual".to_string())).await
    }

    pub async fn find_main_tex_file(&self) -> Result<PathBuf, String> {
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
            if (line.starts_with("!") || 
               line.contains("Error:") || 
               line.contains("error:") ||
               (line.contains(":") && line.contains("undefined"))) 
               && !errors.contains(&line.to_string()) {
                errors.push(line.to_string());
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