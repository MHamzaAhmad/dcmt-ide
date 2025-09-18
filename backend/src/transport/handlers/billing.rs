use axum::{extract::State, Json};
use axum::http::HeaderMap;
use serde::Serialize;
use std::sync::Arc;

use crate::svc::PolarService;

#[derive(Serialize)]
pub struct LimitsResponse {
    pub has_active_subscription: bool,
    pub benefits: Vec<crate::svc::polar_service::BenefitInfo>,
}

pub async fn get_limits(
    State(service): State<Arc<PolarService>>,
    headers: HeaderMap,
) -> Result<Json<LimitsResponse>, axum::http::StatusCode> {
    // Read user id from headers (set by auth layer)
    let user_id = headers
        .get("x-user-id")
        .or_else(|| headers.get("x-userid"))
        .or_else(|| headers.get("x-user"))
        .or_else(|| headers.get("user-id"))
        .or_else(|| headers.get("x-clerk-user-id"))
        .and_then(|v| v.to_str().ok())
        .ok_or(axum::http::StatusCode::FORBIDDEN)?
        .to_string();

    match service.get_user_limits(&user_id).await {
        Ok(res) => Ok(Json(LimitsResponse {
            has_active_subscription: res.has_active_subscription,
            benefits: res.benefits,
        })),
        Err(_) => Err(axum::http::StatusCode::BAD_GATEWAY),
    }
}
