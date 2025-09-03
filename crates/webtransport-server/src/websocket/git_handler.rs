// use tracing::warn;

use super::TransportMessage;
use crate::types::GitOp;

#[cfg(feature = "native-git")]
pub async fn handle_git_operation(operation: GitOp) -> TransportMessage {
    use std::env;
    use tracing::{info, error};
    use crate::git_manager::GitServerManager;
    
    info!("🔧 Handling git operation: {:?}", operation);
    
    // Get workspace path from environment variable
    let workspace_path = env::var("SAMPLE_WORKSPACE_DIR")
        .unwrap_or_else(|_| "/app/workspace/sample".to_string());
    let workspace = std::path::PathBuf::from(&workspace_path);
    
    // Create or get GitServerManager for this workspace
    match GitServerManager::new(workspace) {
        Ok(mut git_manager) => {
            let result = git_manager.execute_operation(operation.clone()).await;
            
            match result {
                Ok(data) => {
                    match &operation {
                        crate::types::GitOp::StartSession => {
                            if let Some(ref response_data) = data {
                                if let crate::types::GitResponseData::SessionBranch(ref branch_name) = response_data {
                                    info!("🌿 Git session started successfully: {}", branch_name);
                                }
                            }
                        }
                        _ => info!("✅ Git operation completed successfully: {:?}", operation),
                    }
                    
                    TransportMessage::GitResponse {
                        success: true,
                        data,
                        error: None,
                    }
                }
                Err(e) => {
                    error!("❌ Git operation failed ({:?}): {}", operation, e);
                    TransportMessage::GitResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to create git manager for workspace '{}': {}", workspace_path, e);
            TransportMessage::GitResponse {
                success: false,
                data: None,
                error: Some(format!("Git manager initialization failed: {}", e)),
            }
        }
    }
}

#[cfg(not(feature = "native-git"))]
pub async fn handle_git_operation(_operation: GitOp) -> TransportMessage {
    warn!("Git operations not available in this build");
    TransportMessage::GitResponse {
        success: false,
        data: None,
        error: Some("Git operations not available in this build".to_string()),
    }
}