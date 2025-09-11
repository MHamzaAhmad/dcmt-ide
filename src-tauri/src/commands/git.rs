use crate::services::git_service::{GitService, CommitSummary};
use crate::repo::git_repository::{GitStatus, GitDiff, CommitResult};
use anyhow::Result;
use std::path::PathBuf;
use tauri::{command, State};
use std::sync::{Arc, Mutex};

pub struct GitServiceState(pub Arc<Mutex<Option<Arc<GitService>>>>);

#[command]
pub async fn initialize_git_service(
    workspace_path: String,
    litellm_base_url: String,
    state: State<'_, GitServiceState>,
) -> Result<(), String> {
    let workspace = PathBuf::from(workspace_path);
    
    match GitService::new(workspace, litellm_base_url) {
        Ok(service) => {
            let mut git_service = state.0.lock().unwrap();
            *git_service = Some(Arc::new(service));
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to initialize git service: {}", e);
            Err(format!("Failed to initialize git service: {}", e))
        }
    }
}

#[command]
pub async fn get_git_status(state: State<'_, GitServiceState>) -> Result<GitStatus, String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.get_status().await
                .map_err(|e| {
                    tracing::error!("Failed to get git status: {}", e);
                    format!("Failed to get git status: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn get_git_diff(
    staged: bool,
    state: State<'_, GitServiceState>,
) -> Result<GitDiff, String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.get_diff(staged).await
                .map_err(|e| {
                    tracing::error!("Failed to get git diff: {}", e);
                    format!("Failed to get git diff: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn generate_commit_summary(
    staged: bool,
    state: State<'_, GitServiceState>,
) -> Result<CommitSummary, String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.generate_commit_summary(staged).await
                .map_err(|e| {
                    tracing::error!("Failed to generate commit summary: {}", e);
                    format!("Failed to generate commit summary: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn stage_files(
    paths: Vec<String>,
    state: State<'_, GitServiceState>,
) -> Result<(), String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.stage_files(paths).await
                .map_err(|e| {
                    tracing::error!("Failed to stage files: {}", e);
                    format!("Failed to stage files: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn stage_all_files(state: State<'_, GitServiceState>) -> Result<(), String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.stage_all().await
                .map_err(|e| {
                    tracing::error!("Failed to stage all files: {}", e);
                    format!("Failed to stage all files: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn commit_changes(
    message: String,
    state: State<'_, GitServiceState>,
) -> Result<CommitResult, String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }
    
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.commit(message).await
                .map_err(|e| {
                    tracing::error!("Failed to commit changes: {}", e);
                    format!("Failed to commit changes: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn push_changes(state: State<'_, GitServiceState>) -> Result<(), String> {
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.push().await
                .map_err(|e| {
                    tracing::error!("Failed to push changes: {}", e);
                    format!("Failed to push changes: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}

#[command]
pub async fn commit_and_push_changes(
    message: String,
    state: State<'_, GitServiceState>,
) -> Result<CommitResult, String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }
    
    let service = {
        let git_service = state.0.lock().unwrap();
        git_service.clone()
    };
    
    match service {
        Some(service) => {
            service.commit_and_push(message).await
                .map_err(|e| {
                    tracing::error!("Failed to commit and push changes: {}", e);
                    format!("Failed to commit and push changes: {}", e)
                })
        }
        None => {
            Err("Git service not initialized".to_string())
        }
    }
}