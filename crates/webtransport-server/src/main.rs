use std::env;
use std::net::SocketAddr;
use std::path::Path;
use serde::{Serialize, Deserialize};
use axum::{
    extract::ws::{WebSocket, Message},
    extract::WebSocketUpgrade,
    response::Response,
    routing::get,
    Router,
};
use tokio::fs;
use futures::{sink::SinkExt, stream::StreamExt};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransportMessage {
    FileOperation { operation: FileOp },
    CompilationRequest { document_id: String, content: String, engine: String },
    CompilationResult { document_id: String, success: bool, pdf_data: Option<Vec<u8>>, log: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileOp {
    Upload { name: String, content: Vec<u8> },
    Download { name: String },
    Delete { name: String },
    List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
        
    let db_path = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:///app/data/latex_ide.db".to_string());
    
    let workspace_path = env::var("WORKSPACE_PATH")
        .unwrap_or_else(|_| "/workspace".to_string());
    
    tracing::info!("Starting WebTransport/WebSocket server on port {}", port);
    tracing::info!("Using database: {}", db_path);
    tracing::info!("Workspace path: {}", workspace_path);
    
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler));
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("WebTransport server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> &'static str {
    "WebTransport/WebSocket server is running"
}

async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_websocket)
}

async fn handle_websocket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    
    tracing::info!("WebSocket connection established");
    
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Binary(data)) = msg {
            match serde_json::from_slice::<TransportMessage>(&data) {
                Ok(transport_msg) => {
                    tracing::info!("Received transport message: {:?}", transport_msg);
                    
                    let response = match transport_msg {
                        TransportMessage::FileOperation { operation } => {
                            handle_file_operation(operation).await
                        }
                        TransportMessage::CompilationRequest { document_id, content, engine } => {
                            handle_compilation_request(document_id, content, engine).await
                        }
                        _ => {
                            tracing::warn!("Unsupported message type");
                            continue;
                        }
                    };
                    
                    if let Ok(response_data) = serde_json::to_vec(&response) {
                        if let Err(e) = sender.send(Message::Binary(response_data)).await {
                            tracing::error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse transport message: {}", e);
                }
            }
        } else if let Ok(Message::Close(_)) = msg {
            tracing::info!("WebSocket connection closed");
            break;
        }
    }
}

async fn handle_file_operation(operation: FileOp) -> TransportMessage {
    // Use workspace path from environment variable
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = Path::new(&workspace_path);
    
    match operation {
        FileOp::List => {
            tracing::info!("Listing files in workspace: {:?}", workspace);
            match list_workspace_files(workspace).await {
                Ok(files) => {
                    tracing::info!("Found {} files in workspace", files.len());
                    // Serialize file list as JSON and return it
                    let files_json = serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string());
                    TransportMessage::FileOperation { 
                        operation: FileOp::Download { name: files_json }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to list workspace files: {}", e);
                    TransportMessage::FileOperation { 
                        operation: FileOp::Download { name: format!("Error: {}", e) }
                    }
                }
            }
        }
        
        FileOp::Download { name } => {
            tracing::info!("Downloading file: {}", name);
            let file_path = workspace.join(&name);
            
            match fs::read(&file_path).await {
                Ok(content) => {
                    tracing::info!("Successfully read file: {} ({} bytes)", name, content.len());
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { 
                            name: "success".to_string(), 
                            content 
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { 
                            name: format!("Error: {}", e), 
                            content: vec![] 
                        }
                    }
                }
            }
        }
        
        FileOp::Upload { name, content } => {
            tracing::info!("Uploading file: {}", name);
            let file_path = workspace.join(&name);
            
            // Ensure parent directory exists
            if let Some(parent) = file_path.parent() {
                let _ = fs::create_dir_all(parent).await;
            }
            
            match fs::write(&file_path, &content).await {
                Ok(_) => {
                    tracing::info!("Successfully uploaded file: {}", name);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { name, content }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to upload file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Upload { name: format!("Error: {}", e), content: vec![] }
                    }
                }
            }
        }
        
        FileOp::Delete { name } => {
            tracing::info!("Deleting file: {}", name);
            let file_path = workspace.join(&name);
            
            match fs::remove_file(&file_path).await {
                Ok(_) => {
                    tracing::info!("Successfully deleted file: {}", name);
                    TransportMessage::FileOperation {
                        operation: FileOp::Delete { name }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to delete file {}: {}", name, e);
                    TransportMessage::FileOperation {
                        operation: FileOp::Delete { name: format!("Error: {}", e) }
                    }
                }
            }
        }
    }
}

async fn handle_compilation_request(document_id: String, content: String, engine: String) -> TransportMessage {
    tracing::info!("Compiling document: {} with engine: {}", document_id, engine);
    
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = Path::new(&workspace_path);
    let tex_file = workspace.join(format!("{}.tex", document_id));
    
    // Write LaTeX content to file
    match fs::write(&tex_file, &content).await {
        Ok(_) => {
            tracing::info!("LaTeX content written to: {:?}", tex_file);
            
            // Run pdflatex compilation
            match compile_latex(&tex_file).await {
                Ok(pdf_data) => {
                    tracing::info!("LaTeX compilation successful for: {}", document_id);
                    TransportMessage::CompilationResult {
                        document_id,
                        success: true,
                        pdf_data: Some(pdf_data),
                        log: "Compilation successful".to_string(),
                    }
                }
                Err(log) => {
                    tracing::error!("LaTeX compilation failed for: {}", document_id);
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
            tracing::error!("Failed to write LaTeX file: {}", e);
            TransportMessage::CompilationResult {
                document_id,
                success: false,
                pdf_data: None,
                log: format!("Failed to write LaTeX file: {}", e),
            }
        }
    }
}

async fn list_workspace_files(workspace_path: &Path) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    
    if !workspace_path.exists() {
        tracing::warn!("Workspace path does not exist: {:?}", workspace_path);
        return Ok(files);
    }
    
    // Recursively scan all files in the workspace
    scan_directory_recursive(workspace_path, workspace_path, &mut files).await?;
    
    // Sort directories first, then files, maintaining hierarchy
    files.sort_by(|a, b| {
        // First sort by directory depth to maintain hierarchy
        let a_depth = a.path.matches('/').count() + a.path.matches('\\').count();
        let b_depth = b.path.matches('/').count() + b.path.matches('\\').count();
        
        match a_depth.cmp(&b_depth) {
            std::cmp::Ordering::Equal => {
                // Same depth - sort directories first, then by name
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
            other => other,
        }
    });
    
    tracing::info!("Found {} total files in workspace (including subdirectories)", files.len());
    
    Ok(files)
}

async fn scan_directory_recursive(
    current_path: &Path, 
    workspace_root: &Path, 
    files: &mut Vec<FileInfo>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut entries = fs::read_dir(current_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Skip hidden files and directories
            if name.starts_with('.') {
                continue;
            }
            
            // Create relative path from workspace root
            let relative_path = path.strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/"); // Normalize path separators
            
            files.push(FileInfo {
                name: name.to_string(),
                path: relative_path,
                is_directory: metadata.is_dir(),
                size: if metadata.is_file() { Some(metadata.len()) } else { None },
                modified: metadata.modified().ok().and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
                }),
            });
            
            // Recursively scan subdirectories
            if metadata.is_dir() {
                Box::pin(scan_directory_recursive(&path, workspace_root, files)).await?;
            }
        }
    }
    
    Ok(())
}

async fn compile_latex(tex_file: &Path) -> Result<Vec<u8>, String> {
    let workspace = tex_file.parent().unwrap_or(Path::new("/app/workspace/sample"));
    let file_name = tex_file.file_stem().unwrap_or(std::ffi::OsStr::new("document"));
    
    tracing::info!("Running pdflatex on: {:?}", tex_file);
    
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
                tracing::info!("Successfully compiled PDF: {:?}", pdf_file);
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