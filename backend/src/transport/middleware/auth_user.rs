use axum::{http::request::Parts, extract::FromRequestParts};
use async_trait::async_trait;
use axum::http::header::AUTHORIZATION;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // The ClerkLayer has already validated the JWT. Extract it and decode payload to get claims.
        let auth = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(axum::http::StatusCode::FORBIDDEN)?;

        let token = auth.strip_prefix("Bearer ").ok_or(axum::http::StatusCode::FORBIDDEN)?;
        let mut segments = token.split('.');
        let _header = segments.next().ok_or(axum::http::StatusCode::FORBIDDEN)?;
        let payload_b64 = segments.next().ok_or(axum::http::StatusCode::FORBIDDEN)?;

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| axum::http::StatusCode::FORBIDDEN)?;
        let claims: Value = serde_json::from_slice(&payload_bytes)
            .map_err(|_| axum::http::StatusCode::FORBIDDEN)?;

        let user_id = claims
            .get("user_id")
            .and_then(|v| v.as_str())
            .or_else(|| claims.get("sub").and_then(|v| v.as_str()))
            .ok_or(axum::http::StatusCode::FORBIDDEN)?
            .to_string();

        Ok(AuthUser { user_id })
    }
}
