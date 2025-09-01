use latex_ide_sse_handler::{
    ModelRequest, ModelResponse, ModelStatus, ModelType
};
use async_trait::async_trait;
use tokio_stream::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub ollama_endpoint: String,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub enable_local_models: bool,
    pub enable_llama: bool,
    pub enable_mistral: bool,
    pub enable_phi: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            ollama_endpoint: "http://localhost:11434".to_string(),
            openai_api_key: None,
            anthropic_api_key: None,
            gemini_api_key: None,
            enable_local_models: false,
            enable_llama: false,
            enable_mistral: false,
            enable_phi: false,
        }
    }
}

/// Enhanced Model Manager with support for multiple AI providers
pub struct ModelManager {
    config: ModelConfig,
    active_models: Arc<RwLock<HashMap<String, Arc<dyn AIModel + Send + Sync>>>>,
    model_metrics: Arc<RwLock<HashMap<String, ModelMetrics>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub average_tokens_per_second: f32,
    pub total_cost: f64,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub error_count: u32,
}

impl Default for ModelMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_tokens: 0,
            average_tokens_per_second: 0.0,
            total_cost: 0.0,
            last_used: None,
            error_count: 0,
        }
    }
}

#[async_trait]
pub trait AIModel {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn model_type(&self) -> ModelType;
    async fn status(&self) -> ModelStatus;
    async fn stream_response(&self, request: ModelRequest) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>>;
    async fn estimate_cost(&self, tokens: u32) -> f64;
    fn context_window(&self) -> u32;
    fn supports_latex(&self) -> bool;
}

impl ModelManager {
    pub async fn new(config: ModelConfig) -> Result<Self> {
        let manager = Self {
            config,
            active_models: Arc::new(RwLock::new(HashMap::new())),
            model_metrics: Arc::new(RwLock::new(HashMap::new())),
        };
        
        Ok(manager)
    }
    
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        let models = self.active_models.read().await;
        let mut model_list = Vec::new();
        
        for (_, model) in models.iter() {
            model_list.push(ModelInfo {
                id: model.id(),
                name: model.name(),
                model_type: model.model_type(),
                status: model.status().await,
                context_window: model.context_window(),
                supports_latex: model.supports_latex(),
            });
        }
        
        model_list
    }
    
    pub async fn get_model(&self, model_id: &str) -> Option<Arc<dyn AIModel + Send + Sync>> {
        let models = self.active_models.read().await;
        models.get(model_id).cloned()
    }
    
    pub async fn stream_response(&self, mut request: ModelRequest) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>> {
        let model_id_clone = request.model_id.clone();
        self.enhance_latex_context(&mut request).await;
        
        let model = self.get_model(&model_id_clone).await
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id_clone))?;
        
        let response_stream = model.stream_response(request).await?;
        
        // Track metrics
        self.update_metrics(&model_id_clone, 1, 0).await;
        
        Ok(response_stream)
    }
    
    async fn enhance_latex_context(&self, _request: &mut ModelRequest) {
        // TODO: Implement LaTeX context enhancement
    }
    
    async fn update_metrics(&self, model_id: &str, requests: u64, tokens: u64) {
        let mut metrics = self.model_metrics.write().await;
        let metric = metrics.entry(model_id.to_string()).or_insert_with(ModelMetrics::default);
        
        metric.total_requests += requests;
        metric.total_tokens += tokens;
        metric.last_used = Some(chrono::Utc::now());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub model_type: ModelType,
    pub status: ModelStatus,
    pub context_window: u32,
    pub supports_latex: bool,
}