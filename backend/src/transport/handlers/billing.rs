use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::svc::PolarService;
use crate::transport::middleware::AuthUser;

#[derive(Serialize)]
pub struct LimitsResponse {
    pub has_active_subscription: bool,
    pub benefits: Vec<crate::svc::polar_service::BenefitInfo>,
}

pub async fn get_limits(
    State(service): State<Arc<PolarService>>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<LimitsResponse>, axum::http::StatusCode> {
    match service.get_user_limits(&user_id).await {
        Ok(res) => Ok(Json(LimitsResponse {
            has_active_subscription: res.has_active_subscription,
            benefits: res.benefits,
        })),
        Err(_) => Err(axum::http::StatusCode::BAD_GATEWAY),
    }
}
