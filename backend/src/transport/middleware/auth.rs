use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

/// Authentication middleware for agent API endpoints
/// 
/// This middleware validates Bearer tokens from the Authorization header.
/// Currently configured to allow requests without authentication for development.
/// In production, set REQUIRE_AUTH=true to enforce authentication.
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Check if authentication is required (default: false for development)
    let require_auth = std::env::var("REQUIRE_AUTH")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    
    if !require_auth {
        debug!("Authentication disabled - allowing request");
        return Ok(next.run(request).await);
    }
    
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    
    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // Validate the token format (basic validation)
            if is_valid_token_format(token) {
                debug!("Valid token format provided");
                return Ok(next.run(request).await);
            } else {
                warn!("Invalid token format provided");
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            warn!("Authorization header missing Bearer prefix");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    
    warn!("No authorization header provided");
    Err(StatusCode::UNAUTHORIZED)
}

/// Validates basic token format
/// In production, this should be replaced with proper JWT validation
fn is_valid_token_format(token: &str) -> bool {
    // Basic validation - token should be non-empty and reasonable length
    !token.is_empty() && token.len() >= 10 && token.len() <= 2048
}