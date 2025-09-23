use axum::{extract::{State, Extension}, Json};
use axum::http::StatusCode;
use serde::Serialize;
use std::sync::Arc;

use crate::svc::PolarService;
use crate::config::Config;
use clerk_rs::validators::authorizer::ClerkJwt;

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
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<LimitsResponse>, StatusCode> {
    // Extract the user ID from the ClerkUser injected by the ClerkLayer
    let user_id = jwt.sub.clone();

    match state.service.get_user_limits(&user_id).await {
        Ok(res) => Ok(Json(LimitsResponse {
            has_active_subscription: res.has_active_subscription,
            benefits: res.benefits,
        })),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

#[derive(Serialize)]
pub struct CheckoutResponse { pub url: String }

pub async fn create_checkout_session(
    State(state): State<BillingState>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<CheckoutResponse>, StatusCode> {
    let user_id = jwt.sub.clone();

    let success_url = format!("{}/?checkout=success", state.config.base_url);
    match state.service.create_checkout_session_url(&user_id, &success_url).await {
        Ok(url) => Ok(Json(CheckoutResponse { url })),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}
