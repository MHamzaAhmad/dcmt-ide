use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, error, debug};

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
                    match compile_latex(&file_to_compile).await {
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

async fn compile_latex(tex_file: &Path) -> Result<Vec<u8>, String> {
    let workspace = tex_file.parent().unwrap_or(Path::new("/app/workspace/sample"));
    let file_name = tex_file.file_stem().unwrap_or(std::ffi::OsStr::new("document"));
    
    // Set up TinyTeX PATH if running in Docker
    let pdflatex_cmd = setup_latex_path();
    
    info!("Running pdflatex on: {:?} with command: {}", tex_file, pdflatex_cmd);
    
    // Use latex-auto script for automatic package installation
    let output = tokio::process::Command::new("latex-auto")
        .arg(tex_file)
        .arg("pdflatex")
        .current_dir(workspace)
        .output()
        .await;
    
    let output = match output {
        Ok(output) => output,
        Err(_) => {
            // Fallback to direct pdflatex if latex-auto is not available
            info!("latex-auto not available, falling back to direct pdflatex");
            tokio::process::Command::new(&pdflatex_cmd)
                .arg("-interaction=nonstopmode")
                .arg("-output-directory")
                .arg(workspace)
                .arg(tex_file)
                .current_dir(workspace)
                .output()
                .await
                .map_err(|e| format!("Failed to run pdflatex ({}): {}", pdflatex_cmd, e))?
        }
    };
    
    if output.status.success() {
        // Read the generated PDF
        let pdf_file = workspace.join(format!("{}.pdf", file_name.to_string_lossy()));
        
        match fs::read(&pdf_file).await {
            Ok(pdf_data) => {
                info!("Successfully compiled PDF: {:?}", pdf_file);
                Ok(pdf_data)
            }
            Err(e) => {
                let log = String::from_utf8_lossy(&output.stdout);
                Err(format!("PDF generation failed: {}. Log: {}", e, log))
            }
        }
    } else {
        let log = String::from_utf8_lossy(&output.stderr);
        Err(format!("pdflatex compilation failed: {}", log))
    }
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

/// Setup LaTeX PATH for TinyTeX installation in Docker
fn setup_latex_path() -> String {
    // Check if we're running in Docker with TinyTeX
    let tinytex_base = Path::new("/root/.TinyTeX/bin");
    
    if tinytex_base.exists() {
        // Find the architecture-specific directory
        if let Ok(entries) = std::fs::read_dir(tinytex_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let pdflatex = path.join("pdflatex");
                    if pdflatex.exists() {
                        debug!("Found pdflatex at: {:?}", pdflatex);
                        
                        // Also update the PATH environment variable for this process
                        if let Ok(current_path) = env::var("PATH") {
                            let new_path = format!("{}:{}", path.display(), current_path);
                            env::set_var("PATH", new_path);
                            debug!("Updated PATH with TinyTeX binaries: {:?}", path);
                        }
                        
                        return pdflatex.to_string_lossy().to_string();
                    }
                }
            }
        }
    }
    
    // Fallback to system pdflatex
    debug!("TinyTeX not found, using system pdflatex");
    "pdflatex".to_string()
}