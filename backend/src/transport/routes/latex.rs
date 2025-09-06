use crate::svc::LaTeXService;
use crate::transport::handlers::latex;
use axum::{
    routing::post,
    Router,
};
use std::sync::Arc;

pub fn latex_router() -> Router<Arc<LaTeXService>> {
    Router::new()
        .route("/compile", post(latex::compile_latex))
}