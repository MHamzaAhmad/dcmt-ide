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

#[debug_handler]
pub async fn find_main_latex_file(
    State(service): State<Arc<LaTeXService>>,
) -> Result<Json<Value>, StatusCode> {
    info!("Finding main LaTeX file request received");
    
    match service.find_main_tex_file().await {
        Ok(tex_file_path) => {
            let relative_path = tex_file_path.strip_prefix(service.get_workspace_path())
                .unwrap_or(&tex_file_path)
                .to_string_lossy()
                .to_string();
            
            info!("Found main LaTeX file: {}", relative_path);
            Ok(Json(json!({
                "success": true,
                "main_file": relative_path
            })))
        }
        Err(e) => {
            error!("Failed to find main LaTeX file: {}", e);
            Ok(Json(json!({
                "success": false,
                "message": e.to_string()
            })))
        }
    }
}

#[debug_handler]
pub async fn latex_status(
    State(service): State<Arc<LaTeXService>>,
) -> Result<Json<Value>, StatusCode> {
    let snapshot = service.get_snapshot().await;
    Ok(Json(json!({
        "success": true,
        "latex": snapshot
    })))
}