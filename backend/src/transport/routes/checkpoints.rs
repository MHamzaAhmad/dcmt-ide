use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::svc::git_service::GitService;
use crate::repo::git_repository::{CheckpointMeta, GitDiff};

#[derive(Debug, Deserialize)]
pub struct ListQuery { pub ns: String, pub max: Option<usize> }

#[derive(Debug, Deserialize)]
pub struct CreateQuery { pub ns: String }

#[derive(Debug, Deserialize)]
pub struct DiffQuery { pub ns: String, pub base: String, pub target: String }

#[derive(Debug, Deserialize)]
pub struct RestoreQuery { pub ns: String, pub id: String }

#[derive(Debug, Deserialize)]
pub struct PublishQuery { pub ns: String }

#[derive(Debug, Deserialize)]
pub struct CreateBody { pub title: Option<String>, pub reason: Option<String>, pub actor: Option<String> }

#[derive(Debug, Deserialize)]
pub struct RestoreBody { pub message: Option<String> }

#[derive(Debug, Serialize)]
pub struct ListResponse { pub checkpoints: Vec<CheckpointMeta> }

#[derive(Debug, Serialize)]
pub struct CreateResponse { pub checkpoint: CheckpointMeta }

#[derive(Debug, Serialize)]
pub struct DiffResponse { pub diff: GitDiff }

#[derive(Debug, Serialize)]
pub struct RestoreResponse { pub result: String, #[serde(rename = "commitId")] pub commit_id: Option<String> }

#[derive(Debug, Serialize)]
pub struct PublishResponse { pub result: String, #[serde(rename = "mergeCommitId")] pub merge_commit_id: Option<String> }

#[derive(Debug, Serialize)]
pub struct ErrorResponse { pub error: String }

pub fn checkpoints_router() -> Router<Arc<GitService>> {
    Router::new()
        .route("/", get(list_checkpoints).post(create_checkpoint))
        .route("/diff", get(diff_checkpoint))
        .route("/restore", post(restore_checkpoint))
        .route("/publish", post(publish_checkpoints))
}

async fn list_checkpoints(
    State(svc): State<Arc<GitService>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, (StatusCode, Json<ErrorResponse>)> {
    match svc.checkpoints_list(&q.ns, q.max).await {
        Ok(items) => Ok(Json(ListResponse { checkpoints: items })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn create_checkpoint(
    State(svc): State<Arc<GitService>>,
    Query(q): Query<CreateQuery>,
    Json(body): Json<CreateBody>,
) -> Result<Json<CreateResponse>, (StatusCode, Json<ErrorResponse>)> {
    match svc.checkpoints_create(&q.ns, body.title).await {
        Ok(cp) => Ok(Json(CreateResponse { checkpoint: cp })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn diff_checkpoint(
    State(svc): State<Arc<GitService>>,
    Query(q): Query<DiffQuery>,
) -> Result<Json<DiffResponse>, (StatusCode, Json<ErrorResponse>)> {
    match svc.checkpoints_diff(&q.ns, &q.base, &q.target).await {
        Ok(diff) => Ok(Json(DiffResponse { diff })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn restore_checkpoint(
    State(svc): State<Arc<GitService>>,
    Query(q): Query<RestoreQuery>,
    Json(body): Json<RestoreBody>,
) -> Result<Json<RestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    match svc.checkpoints_restore(&q.ns, &q.id, body.message).await {
        Ok(commit) => Ok(Json(RestoreResponse { result: "ok".into(), commit_id: Some(commit.sha) })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn publish_checkpoints(
    State(svc): State<Arc<GitService>>,
    Query(q): Query<PublishQuery>,
) -> Result<Json<PublishResponse>, (StatusCode, Json<ErrorResponse>)> {
    match svc.checkpoints_publish(&q.ns).await {
        Ok(oid) => Ok(Json(PublishResponse { result: "ok".into(), merge_commit_id: Some(oid) })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}
