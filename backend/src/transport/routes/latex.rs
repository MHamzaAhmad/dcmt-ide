use crate::svc::LaTeXService;
use crate::transport::handlers::latex;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn latex_router() -> Router<Arc<LaTeXService>> {
    Router::new()
        .route("/compile", post(latex::compile_latex))
        .route("/find-main", get(latex::find_main_latex_file))
}