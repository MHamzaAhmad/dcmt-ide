use crate::svc::FileService;
use crate::transport::handlers::websocket;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn websocket_router() -> Router<Arc<FileService>> {
    Router::new()
        .route("/", get(websocket::websocket_handler))
}