use clerk_rs::{
    clerk::Clerk,
    validators::{axum::ClerkLayer, jwks::MemoryCacheJwksProvider},
    ClerkConfiguration,
};
use tracing::debug;

/// Creates Clerk authentication layer for protecting API endpoints
///
/// This function creates a Clerk authentication middleware that validates JWT tokens
/// from the Authorization header. Returns 403 Forbidden for invalid/missing tokens.
pub fn create_clerk_auth_layer(clerk_secret_key: Option<String>) -> ClerkLayer<MemoryCacheJwksProvider> {
    // Get the secret key, panic if not provided since auth is required
    let secret_key = clerk_secret_key
        .expect("CLERK_SECRET_KEY is required for authentication");

    debug!("Creating Clerk authentication layer");

    // Create Clerk configuration with the secret key
    let config = ClerkConfiguration::new(
        None,                           // publishable_key (not needed for backend validation)
        None,                           // jwt_key (not needed when using secret key)
        Some(secret_key),              // secret_key
        None,                          // api_url (use default)
    );

    // Initialize Clerk client
    let clerk = Clerk::new(config);

    // Create the authentication layer with:
    // - MemoryCacheJwksProvider for JWT validation
    // - None to protect all routes (no specific route filtering)
    // - true to validate session tokens
    ClerkLayer::new(
        MemoryCacheJwksProvider::new(clerk),
        None,
        true,
    )
}