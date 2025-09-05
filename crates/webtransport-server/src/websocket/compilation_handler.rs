use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, error, debug};
use regex::Regex;

use super::TransportMessage;

pub async fn handle_compilation_request(document_id: String, content: String, engine: String) -> TransportMessage {
    info!("Compiling document: {} with engine: {}", document_id, engine);
    
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = Path::new(&workspace_path);
    
    // Ensure workspace directory exists
    if !workspace.exists() {
        if let Err(e) = fs::create_dir_all(workspace).await {
            error!("Failed to create workspace directory: {}", e);
            return TransportMessage::CompilationResult {
                document_id,
                success: false,
                pdf_data: None,
                log: format!("Failed to create workspace directory: {}", e),
            };
        }
        debug!("Created workspace directory: {:?}", workspace);
    }
    
    // Clean up the document_id (remove .tex if present)
    let clean_id = document_id.trim_end_matches(".tex");
    let tex_file = workspace.join(format!("{}.tex", clean_id));
    
    // Write the current content to file
    match fs::write(&tex_file, &content).await {
        Ok(_) => {
            info!("LaTeX content written to: {:?}", tex_file);
            
            // Determine which file to compile (main entry point or current file)
            match find_main_document(&tex_file, workspace, &content).await {
                Ok(file_to_compile) => {
                    info!("Found main document to compile: {:?}", file_to_compile);
                    
                    // Run pdflatex compilation
                    match compile_latex(&file_to_compile, &engine).await {
                        Ok(pdf_data) => {
                            info!("LaTeX compilation successful for: {}", document_id);
                            TransportMessage::CompilationResult {
                                document_id,
                                success: true,
                                pdf_data: Some(pdf_data),
                                log: "Compilation successful".to_string(),
                            }
                        }
                        Err(log) => {
                            error!("LaTeX compilation failed for: {}", document_id);
                            TransportMessage::CompilationResult {
                                document_id,
                                success: false,
                                pdf_data: None,
                                log,
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Cannot compile {}: {}", document_id, e);
                    TransportMessage::CompilationResult {
                        document_id,
                        success: false,
                        pdf_data: None,
                        log: e,
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to write LaTeX file: {}", e);
            TransportMessage::CompilationResult {
                document_id,
                success: false,
                pdf_data: None,
                log: format!("Failed to write LaTeX file: {}", e),
            }
        }
    }
}

async fn compile_latex(tex_file: &Path, user_engine: &str) -> Result<Vec<u8>, String> {
    let workspace = tex_file.parent().unwrap_or(Path::new("/app/workspace/sample"));
    let file_name = tex_file.file_stem().unwrap_or(std::ffi::OsStr::new("document"));
    
    info!("Compiling LaTeX file: {:?}", tex_file);
    
    // Try latexmk first (recommended approach)
    let output = match try_latexmk(tex_file, workspace, user_engine).await {
        Ok(output) => Ok(output),
        Err(_) => try_pdflatex_direct(tex_file, workspace, user_engine).await,
    };
    
    match output {
        Ok(output) => {
            if output.status.success() {
                // Read the generated PDF
                let pdf_file = workspace.join(format!("{}.pdf", file_name.to_string_lossy()));
                
                match fs::read(&pdf_file).await {
                    Ok(pdf_data) => {
                        info!("Successfully compiled PDF: {:?}", pdf_file);
                        Ok(pdf_data)
                    }
                    Err(e) => {
                        let stdout_log = String::from_utf8_lossy(&output.stdout);
                        let stderr_log = String::from_utf8_lossy(&output.stderr);
                        Err(format!("PDF file not found after compilation: {}. Stdout: {}. Stderr: {}", e, stdout_log, stderr_log))
                    }
                }
            } else {
                let stdout_log = String::from_utf8_lossy(&output.stdout);
                let stderr_log = String::from_utf8_lossy(&output.stderr);
                
                // Parse for specific error patterns and provide helpful messages
                let error_message = if stdout_log.contains("fontspec") && stdout_log.contains("XeTeX or") {
                    format!("LaTeX Engine Error: This document uses fontspec package which requires XeLaTeX or LuaLaTeX.\n\nThe document contains font-related packages that are not compatible with pdfLaTeX. Please ensure your document structure is compatible or the auto-detection is working properly.\n\nDetailed error:\n{}", stdout_log)
                } else if stdout_log.contains("File `(") && stdout_log.contains("not found") {
                    format!("LaTeX Syntax Error: Invalid command syntax detected.\n\nLooks like there's a syntax error in your LaTeX code. Please check for missing braces or incorrect command usage.\n\nDetailed error:\n{}", stdout_log)
                } else if stdout_log.contains("Package") && stdout_log.contains("Error") {
                    format!("LaTeX Package Error: A required package is missing or incompatible with the current engine.\n\nDetailed error:\n{}", stdout_log)
                } else if stdout_log.contains("Bib file(s) not found") {
                    format!("LaTeX Bibliography Error: Bibliography file is missing.\n\nThe document references a bibliography file that couldn't be found. Either create the .bib file or remove the bibliography commands.\n\nDetailed error:\n{}", stdout_log)
                } else if stdout_log.contains("undefined references") {
                    format!("LaTeX Reference Warning: Document compiled but has undefined references.\n\nThe PDF was generated successfully but some citations or references are undefined. This is usually not critical for viewing.\n\nDetailed log:\n{}", stdout_log)
                } else {
                    format!("LaTeX Compilation failed:\n\nOutput: {}\nErrors: {}", stdout_log, stderr_log)
                };
                
                Err(error_message)
            }
        }
        Err(e) => Err(format!("Failed to run LaTeX compiler: {}", e))
    }
}

/// Try compiling with latexmk (recommended)
async fn try_latexmk(tex_file: &Path, _workspace: &Path, user_engine: &str) -> Result<std::process::Output, String> {
    info!("Attempting compilation with latexmk");
    
    // Detect the appropriate engine based on file content and user preference
    let engine = resolve_latex_engine(tex_file, user_engine).await;
    info!("Using LaTeX engine: {}", engine);
    
    let mut cmd = tokio::process::Command::new("latexmk");
    cmd.arg("-synctex=1")             // Enable SyncTeX for source-PDF sync
        .arg("-interaction=nonstopmode") // Don't stop on errors  
        .arg("-file-line-error")       // Better error formatting
        .arg("-g")                     // Force regeneration (like -f but keeps cache)
        .arg("-cd")                    // Change to document directory before processing
        .arg(tex_file);
    
    // Set engine-specific flags
    match engine.as_str() {
        "xelatex" => {
            cmd.arg("-xelatex");       // Use XeLaTeX engine
        }
        "lualatex" => {
            cmd.arg("-lualatex");      // Use LuaLaTeX engine  
        }
        "latex" => {
            cmd.arg("-latex");         // Use LaTeX->dvips->ps2pdf chain
        }
        _ => {
            cmd.arg("-pdf");           // Use pdfLaTeX (default)
        }
    }
    
    cmd.output()
        .await
        .map_err(|e| format!("Failed to run latexmk with {}: {}", engine, e))
}

/// Fallback to direct engine when latexmk is not available
async fn try_pdflatex_direct(tex_file: &Path, workspace: &Path, user_engine: &str) -> Result<std::process::Output, String> {
    info!("latexmk not available, falling back to direct engine");
    
    // Detect the appropriate engine based on file content and user preference
    let engine = resolve_latex_engine(tex_file, user_engine).await;
    info!("Using direct engine: {}", engine);
    
    let engine_cmd = match engine.as_str() {
        "xelatex" => "xelatex",
        "lualatex" => "lualatex", 
        "latex" => "latex",
        _ => "pdflatex",
    };
    
    tokio::process::Command::new(engine_cmd)
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg("-file-line-error")
        .arg("-output-directory")
        .arg(workspace)
        .arg(tex_file)
        .current_dir(workspace)
        .output()
        .await
        .map_err(|e| format!("Failed to run {}: {}", engine_cmd, e))
}

/// Find the main LaTeX document to compile
async fn find_main_document(current_file: &Path, workspace: &Path, content: &str) -> Result<PathBuf, String> {
    // Check if the current file itself is a complete document
    if is_complete_document(content) {
        debug!("Current file is a complete document");
        return Ok(current_file.to_path_buf());
    }
    
    // Look for main.tex or document.tex in the workspace
    let main_candidates = ["main.tex", "document.tex", "thesis.tex", "report.tex"];
    
    for candidate in &main_candidates {
        let main_path = workspace.join(candidate);
        if main_path.exists() && main_path != current_file {
            // Check if this main file includes the current file
            if let Ok(main_content) = fs::read_to_string(&main_path).await {
                let current_name = current_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                
                // Check for \input{filename} or \include{filename}
                if main_content.contains(&format!("\\input{{{}", current_name))
                    || main_content.contains(&format!("\\include{{{}", current_name))
                    || main_content.contains(&format!("\\input{{{}.tex", current_name))
                    || main_content.contains(&format!("\\include{{{}.tex", current_name)) {
                    info!("Found main document {} that includes current file", candidate);
                    return Ok(main_path);
                }
            }
        }
    }
    
    // If no main document found and current is not complete, check if there's ANY main.tex
    let main_tex = workspace.join("main.tex");
    if main_tex.exists() && main_tex != current_file {
        info!("Using main.tex as fallback compilation target");
        return Ok(main_tex);
    }
    
    // Error: no compilable document found
    Err(format!(
        "Cannot compile: The file '{}' appears to be a LaTeX fragment (chapter/section) without a complete document structure. \
        No main document (main.tex, document.tex, etc.) found in the workspace that includes this file. \
        Please either: 1) Open and compile the main document, or 2) Add document structure (\\documentclass, \\begin{{document}}, etc.) to make this file standalone.",
        current_file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")
    ))
}

/// Check if content is a complete LaTeX document
fn is_complete_document(content: &str) -> bool {
    content.contains("\\documentclass") && 
    content.contains("\\begin{document}") && 
    content.contains("\\end{document}")
}

/// Resolve the LaTeX engine based on user preference and auto-detection
async fn resolve_latex_engine(tex_file: &Path, user_engine: &str) -> String {
    // If user specified a specific engine (not "auto"), use it
    match user_engine {
        "pdflatex" | "xelatex" | "lualatex" | "latex" => {
            info!("Using user-specified engine: {}", user_engine);
            user_engine.to_string()
        }
        "auto" | _ => {
            // Auto-detect based on file content
            detect_latex_engine(tex_file).await
        }
    }
}

/// Detect the appropriate LaTeX engine based on file content
async fn detect_latex_engine(tex_file: &Path) -> String {
    let content = match fs::read_to_string(tex_file).await {
        Ok(content) => content,
        Err(_) => return "pdflatex".to_string(), // Default fallback
    };
    
    // Check for packages that require XeLaTeX or LuaLaTeX
    let fontspec_regex = Regex::new(r"\\usepackage.*\{fontspec\}").unwrap();
    let polyglossia_regex = Regex::new(r"\\usepackage.*\{polyglossia\}").unwrap();
    let unicode_math_regex = Regex::new(r"\\usepackage.*\{unicode-math\}").unwrap();
    let font_commands_regex = Regex::new(r"\\set(main|sans|mono)font").unwrap();
    
    if fontspec_regex.is_match(&content) ||
       polyglossia_regex.is_match(&content) ||
       unicode_math_regex.is_match(&content) ||
       font_commands_regex.is_match(&content) {
        
        // Check for LuaLaTeX-specific packages
        let luaotfload_regex = Regex::new(r"\\usepackage.*\{luaotfload\}").unwrap();
        let luacode_regex = Regex::new(r"\\usepackage.*\{luacode\}").unwrap();
        let directlua_regex = Regex::new(r"\\directlua").unwrap();
        
        if luaotfload_regex.is_match(&content) ||
           luacode_regex.is_match(&content) ||
           directlua_regex.is_match(&content) {
            info!("Detected LuaLaTeX due to Lua-specific packages");
            return "lualatex".to_string();
        }
        
        info!("Detected XeLaTeX due to fontspec/unicode packages");
        return "xelatex".to_string();
    }
    
    // Check for PSTricks which works best with LaTeX->dvips->ps2pdf
    let pstricks_regex = Regex::new(r"\\usepackage.*\{pstricks\}").unwrap();
    let pst_regex = Regex::new(r"\\usepackage.*\{pst-").unwrap();
    
    if pstricks_regex.is_match(&content) || pst_regex.is_match(&content) {
        info!("Detected LaTeX due to PSTricks packages");
        return "latex".to_string();
    }
    
    // Check for other LuaLaTeX-specific packages
    let luamplib_regex = Regex::new(r"\\usepackage.*\{luamplib\}").unwrap();
    if luamplib_regex.is_match(&content) {
        info!("Detected LuaLaTeX due to luamplib package");
        return "lualatex".to_string();
    }
    
    // Default to pdfLaTeX for compatibility
    info!("Using pdfLaTeX as default engine");
    "pdflatex".to_string()
}

