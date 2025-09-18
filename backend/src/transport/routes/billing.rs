use axum::{routing::get, Router};
use crate::transport::handlers::billing::get_limits;
use std::sync::Arc;
use crate::svc::PolarService;

pub fn billing_router() -> Router<Arc<PolarService>> {
    Router::new().route("/limits", get(get_limits))
}
