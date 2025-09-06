use crate::model::LaTeXCompileRequest;
use crate::svc::LaTeXService;
use axum::{
    debug_handler,
    extract::State,
    http::StatusCode,
    response::Json,
    Json as RequestJson,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

#[debug_handler]
pub async fn compile_latex(
    State(service): State<Arc<LaTeXService>>,
    RequestJson(request): RequestJson<LaTeXCompileRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("LaTeX compilation request received with provider: {:?}", request.provider);
    
    match service.compile_workspace(request).await {
        Ok(response) => {
            if response.success {
                info!("LaTeX compilation completed successfully");
                Ok(Json(json!({
                    "success": true,
                    "message": response.message,
                    "output_file": response.output_file
                })))
            } else {
                error!("LaTeX compilation failed: {}", response.message);
                Ok(Json(json!({
                    "success": false,
                    "message": response.message,
                    "errors": response.errors.unwrap_or_default()
                })))
            }
        }
        Err(e) => {
            error!("LaTeX compilation service error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}