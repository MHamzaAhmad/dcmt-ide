use axum::{routing::{get, post}, Router};
use crate::transport::handlers::billing::{get_limits, create_checkout_session, BillingState};
 

pub fn billing_router() -> Router<BillingState> {
    Router::new()
        .route("/limits", get(get_limits))
        .route("/checkout", post(create_checkout_session))
}
