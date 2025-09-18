use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::svc::git_service::{CommitSummary, GitService};
use crate::repo::git_repository::{CommitResult, GitDiff, GitStatus};

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    staged: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StageFilesRequest {
    paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitAndPushRequest {
    message: String,
}

#[derive(Debug, Serialize)]
pub struct GitStatusResponse {
    status: GitStatus,
}

#[derive(Debug, Serialize)]
pub struct GitDiffResponse {
    diff: GitDiff,
}

#[derive(Debug, Serialize)]
pub struct CommitSummaryResponse {
    summary: CommitSummary,
}

#[derive(Debug, Serialize)]
pub struct StageFilesResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct CommitResponse {
    commit: CommitResult,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub fn git_router() -> Router<Arc<GitService>> {
    Router::new()
        .route("/status", get(get_git_status))
        .route("/diff", get(get_git_diff))
        .route("/summary", post(generate_commit_summary))
        .route("/stage", post(stage_files))
        .route("/stage-all", post(stage_all))
        .route("/commit", post(commit_changes))
        .route("/push", post(push_changes))
        .route("/commit-and-push", post(commit_and_push))
}

async fn get_git_status(
    State(git_service): State<Arc<GitService>>,
) -> Result<Json<GitStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    match git_service.get_status().await {
        Ok(status) => Ok(Json(GitStatusResponse { status })),
        Err(e) => {
            tracing::error!("Failed to get git status: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to get git status: {}", e),
                }),
            ))
        }
    }
}

async fn get_git_diff(
    Query(params): Query<DiffQuery>,
    State(git_service): State<Arc<GitService>>,
) -> Result<Json<GitDiffResponse>, (StatusCode, Json<ErrorResponse>)> {
    let staged = params.staged.unwrap_or(false);
    
    match git_service.get_diff(staged).await {
        Ok(diff) => Ok(Json(GitDiffResponse { diff })),
        Err(e) => {
            tracing::error!("Failed to get git diff: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to get git diff: {}", e),
                }),
            ))
        }
    }
}

async fn generate_commit_summary(
    Query(params): Query<DiffQuery>,
    State(git_service): State<Arc<GitService>>,
) -> Result<Json<CommitSummaryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let staged = params.staged.unwrap_or(false);
    let started = Instant::now();
    tracing::info!("Summary request received: staged={}", staged);

    match git_service.generate_commit_summary(staged).await {
        Ok(summary) => {
            let elapsed = started.elapsed().as_millis();
            let bullets = summary.bullets.len();
            let summary_len = summary.summary.len();
            tracing::info!(
                "Summary success: staged={}, duration_ms={}, bullets={}, summary_chars={}",
                staged, elapsed, bullets, summary_len
            );
            Ok(Json(CommitSummaryResponse { summary }))
        }
        Err(e) => {
            let elapsed = started.elapsed().as_millis();
            tracing::error!(
                "Summary error: staged={}, duration_ms={}, error={}",
                staged, elapsed, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to generate commit summary: {}", e),
                }),
            ))
        }
    }
}

async fn stage_files(
    State(git_service): State<Arc<GitService>>,
    Json(request): Json<StageFilesRequest>,
) -> Result<Json<StageFilesResponse>, (StatusCode, Json<ErrorResponse>)> {
    match git_service.stage_files(request.paths).await {
        Ok(()) => Ok(Json(StageFilesResponse {
            success: true,
            message: "Files staged successfully".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to stage files: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to stage files: {}", e),
                }),
            ))
        }
    }
}

async fn stage_all(
    State(git_service): State<Arc<GitService>>,
) -> Result<Json<StageFilesResponse>, (StatusCode, Json<ErrorResponse>)> {
    match git_service.stage_all().await {
        Ok(()) => Ok(Json(StageFilesResponse {
            success: true,
            message: "All files staged successfully".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to stage all files: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to stage all files: {}", e),
                }),
            ))
        }
    }
}

async fn commit_changes(
    State(git_service): State<Arc<GitService>>,
    Json(request): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Commit message cannot be empty".to_string(),
            }),
        ));
    }
    
    match git_service.commit(request.message).await {
        Ok(commit) => Ok(Json(CommitResponse { commit })),
        Err(e) => {
            tracing::error!("Failed to commit changes: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to commit changes: {}", e),
                }),
            ))
        }
    }
}

async fn push_changes(
    State(git_service): State<Arc<GitService>>,
) -> Result<Json<PushResponse>, (StatusCode, Json<ErrorResponse>)> {
    match git_service.push().await {
        Ok(()) => Ok(Json(PushResponse {
            success: true,
            message: "Changes pushed successfully".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to push changes: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to push changes: {}", e),
                }),
            ))
        }
    }
}

async fn commit_and_push(
    State(git_service): State<Arc<GitService>>,
    Json(request): Json<CommitAndPushRequest>,
) -> Result<Json<CommitResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Commit message cannot be empty".to_string(),
            }),
        ));
    }
    
    match git_service.commit_and_push(request.message).await {
        Ok(commit) => Ok(Json(CommitResponse { commit })),
        Err(e) => {
            tracing::error!("Failed to commit and push changes: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to commit and push changes: {}", e),
                }),
            ))
        }
    }
}