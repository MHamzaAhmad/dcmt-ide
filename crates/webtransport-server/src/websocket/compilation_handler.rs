use std::env;
use std::path::Path;
use tokio::fs;
use tracing::{info, error};

use super::TransportMessage;

pub async fn handle_compilation_request(document_id: String, content: String, engine: String) -> TransportMessage {
    info!("Compiling document: {} with engine: {}", document_id, engine);
    
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = Path::new(&workspace_path);
    let tex_file = workspace.join(format!("{}.tex", document_id));
    
    // Write LaTeX content to file
    match fs::write(&tex_file, &content).await {
        Ok(_) => {
            info!("LaTeX content written to: {:?}", tex_file);
            
            // Run pdflatex compilation
            match compile_latex(&tex_file).await {
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
    
    info!("Running pdflatex on: {:?}", tex_file);
    
    let output = tokio::process::Command::new("pdflatex")
        .arg("-interaction=nonstopmode")
        .arg("-output-directory")
        .arg(workspace)
        .arg(tex_file)
        .current_dir(workspace)
        .output()
        .await
        .map_err(|e| format!("Failed to run pdflatex: {}", e))?;
    
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