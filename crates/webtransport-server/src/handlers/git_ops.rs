use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use wtransport::RecvStream;
use anyhow::Result;
use tracing::{debug, warn, error};

use crate::types::{WebTransportMessage, GitResponseData};
use crate::utils::read_message;
use crate::git_manager::GitServerManager;

pub async fn handle_git_ops_stream(
    stream: &mut RecvStream,
    git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
) -> Result<()> {
    let message = read_message(stream).await?;
    
    match message {
        WebTransportMessage::GitOperation { operation } => {
            debug!("Git operation request: {:?}", operation);
            
            let response = execute_git_operation(operation, git_managers).await;
            
            // Send response back (for now we'll just log it)
            match response {
                Ok(data) => {
                    debug!("Git operation completed successfully: {:?}", data);
                    // TODO: Send GitResponse back to client
                }
                Err(e) => {
                    error!("Git operation failed: {}", e);
                    // TODO: Send error GitResponse back to client
                }
            }
        }
        _ => {
            warn!("Unexpected message on git ops stream: {:?}", message);
        }
    }
    
    Ok(())
}

async fn execute_git_operation(
    operation: crate::types::GitOp,
    git_managers: Arc<RwLock<HashMap<String, Arc<Mutex<GitServerManager>>>>>,
) -> Result<Option<GitResponseData>> {
    use std::path::PathBuf;
    
    // For now, we'll use current directory as workspace. In production, 
    // this should be determined from the client session context.
    let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace_key = workspace_path.to_string_lossy().to_string();
    
    // Get or create git manager for this workspace
    let git_manager = {
        let mut managers = git_managers.write().await;
        managers.entry(workspace_key.clone())
            .or_insert_with(|| {
                match GitServerManager::new(workspace_path.clone()) {
                    Ok(manager) => Arc::new(Mutex::new(manager)),
                    Err(e) => {
                        error!("Failed to create git manager: {}", e);
                        // Return a dummy manager that will fail operations
                        Arc::new(Mutex::new(GitServerManager::dummy(workspace_path)))
                    }
                }
            })
            .clone()
    };
    
    let mut manager = git_manager.lock().await;
    manager.execute_operation(operation).await
}