use crate::model::{LaTeXCompileRequest, LaTeXCompileResponse, LaTeXProvider, CompilationEvent, FileEvent};
use crate::repo::LaTeXRepository;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock, broadcast};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

pub type CompilationEventSender = broadcast::Sender<CompilationEvent>;
pub type CompilationEventReceiver = broadcast::Receiver<CompilationEvent>;

#[derive(Clone)]
pub struct LaTeXService {
    repository: Arc<LaTeXRepository>,
    compilation_lock: Arc<Mutex<()>>,
    compilation_event_sender: CompilationEventSender,
    main_tex_file: Arc<RwLock<Option<PathBuf>>>,
    debounce_timers: Arc<RwLock<HashMap<String, Instant>>>,
    is_auto_compile_enabled: Arc<RwLock<bool>>,
}

impl LaTeXService {
    pub fn new(workspace_path: PathBuf) -> Self {
        let repository = Arc::new(LaTeXRepository::new(workspace_path));
        let compilation_lock = Arc::new(Mutex::new(()));
        let (compilation_event_sender, _) = broadcast::channel(1000);

        Self {
            repository,
            compilation_lock,
            compilation_event_sender,
            main_tex_file: Arc::new(RwLock::new(None)),
            debounce_timers: Arc::new(RwLock::new(HashMap::new())),
            is_auto_compile_enabled: Arc::new(RwLock::new(true)),
        }
    }

    pub fn subscribe_to_compilation_events(&self) -> CompilationEventReceiver {
        self.compilation_event_sender.subscribe()
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
    pub async fn handle_file_change(&self, file_event: &FileEvent) {
        if !self.should_trigger_compilation(&file_event.path) {
            return;
        }

        if !*self.is_auto_compile_enabled.read().await {
            debug!("Auto-compilation disabled, ignoring file change: {}", file_event.path);
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
                format!("file_change:{}", file_event.path)
            );
            let _ = self.compilation_event_sender.send(queued_event);

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
    pub async fn detect_and_set_main_file(&self) -> Result<()> {
        match self.repository.find_main_tex_file().await {
            Ok(main_file) => {
                let main_file_str = main_file.strip_prefix(&self.repository.workspace_path)
                    .unwrap_or(&main_file)
                    .to_string_lossy()
                    .to_string();
                
                *self.main_tex_file.write().await = Some(main_file.clone());
                
                // Emit main file detected event
                let event = CompilationEvent::main_file_detected(main_file_str);
                let _ = self.compilation_event_sender.send(event);
                
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
        
        let main_file_str = main_file.strip_prefix(&self.repository.workspace_path)
            .unwrap_or(&main_file)
            .to_string_lossy()
            .to_string();
        
        let event = CompilationEvent::main_file_detected(main_file_str);
        let _ = self.compilation_event_sender.send(event);
        
        info!("Main LaTeX file manually set: {:?}", main_file);
    }

    pub async fn compile_workspace(&self, request: LaTeXCompileRequest) -> Result<LaTeXCompileResponse> {
        self.compile_workspace_internal(request, None).await
    }

    /// Internal compilation method with event emission
    async fn compile_workspace_internal(&self, request: LaTeXCompileRequest, _reason: Option<String>) -> Result<LaTeXCompileResponse> {
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
        if !self.repository.check_latexmk_available().await {
            let error_response = LaTeXCompileResponse::error(
                "latexmk is not available".to_string(),
                vec!["Please install latexmk to compile LaTeX documents".to_string()],
            );
            return Ok(error_response);
        }

        // Find main tex file
        let main_tex_file = match self.repository.find_main_tex_file().await {
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

        let main_file_str = main_tex_file.strip_prefix(&self.repository.workspace_path)
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
                    if self.repository.check_engine_available(engine).await {
                        vec![engine.to_string()]
                    } else {
                        let error_event = CompilationEvent::error(
                            main_file_str.clone(),
                            vec![format!("Specified engine '{}' is not available", engine)],
                            Some(engine.to_string())
                        );
                        let _ = self.compilation_event_sender.send(error_event);

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
            let _ = self.compilation_event_sender.send(error_event);

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
            let _ = self.compilation_event_sender.send(started_event);
            
            match self.repository.execute_latexmk(&main_tex_file, engine).await {
                Ok((_stdout, _stderr)) => {
                    // Compilation succeeded
                    let pdf_path = self.repository.extract_pdf_path(&main_tex_file);
                    let relative_pdf_path = pdf_path.strip_prefix(&self.repository.workspace_path)
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
                    let _ = self.compilation_event_sender.send(success_event);
                    
                    return Ok(LaTeXCompileResponse::success(
                        format!("Compilation successful with {}", engine),
                        Some(relative_pdf_path),
                    ));
                }
                Err(e) => {
                    warn!("Compilation failed with engine {}: {}", engine, e);
                    
                    // Get raw LaTeX output for detailed error information
                    let error_output = e.to_string();
                    let raw_errors = LaTeXRepository::get_raw_latex_output(&error_output, "");
                    last_errors = raw_errors;
                    
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
        let _ = self.compilation_event_sender.send(error_event);
        
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

    /// Handle startup compilation
    pub async fn handle_startup_compilation(&self) -> Result<()> {
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
    pub async fn handle_agent_completion(&self) -> Result<()> {
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
            let _ = self.compilation_event_sender.send(queued_event);

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
    pub async fn force_compile(&self) -> Result<LaTeXCompileResponse> {
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

    pub fn get_workspace_path(&self) -> &PathBuf {
        &self.repository.workspace_path
    }
}