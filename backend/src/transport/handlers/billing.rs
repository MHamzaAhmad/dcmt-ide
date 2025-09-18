use axum::{extract::State, Json};
use axum::http::HeaderMap;
use serde::Serialize;
use std::sync::Arc;

use crate::svc::PolarService;
use crate::config::Config;

#[derive(Clone)]
pub struct BillingState {
    pub service: Arc<PolarService>,
    pub config: Config,
}

#[derive(Serialize)]
pub struct LimitsResponse {
    pub has_active_subscription: bool,
    pub benefits: Vec<crate::svc::polar_service::BenefitInfo>,
}

pub async fn get_limits(
    State(state): State<BillingState>,
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

    match state.service.get_user_limits(&user_id).await {
        Ok(res) => Ok(Json(LimitsResponse {
            has_active_subscription: res.has_active_subscription,
            benefits: res.benefits,
        })),
        Err(_) => Err(axum::http::StatusCode::BAD_GATEWAY),
    }
}

#[derive(Serialize)]
pub struct CheckoutResponse { pub url: String }

pub async fn create_checkout_session(
    State(state): State<BillingState>,
    headers: HeaderMap,
) -> Result<Json<CheckoutResponse>, axum::http::StatusCode> {
    let user_id = headers
        .get("x-user-id")
        .or_else(|| headers.get("x-userid"))
        .or_else(|| headers.get("x-user"))
        .or_else(|| headers.get("user-id"))
        .or_else(|| headers.get("x-clerk-user-id"))
        .and_then(|v| v.to_str().ok())
        .ok_or(axum::http::StatusCode::FORBIDDEN)?
        .to_string();

    let success_url = format!("{}/?checkout=success", state.config.base_url);
    match state.service.create_checkout_session_url(&user_id, &success_url).await {
        Ok(url) => Ok(Json(CheckoutResponse { url })),
        Err(_) => Err(axum::http::StatusCode::BAD_GATEWAY),
    }
}
