use latex_ide_sse_handler::{
    LocalModel, RemoteModel, ModelManager as SSEModelManager, 
    ModelRequest, ModelResponse, ModelStatus, ModelType
};
use kalosm::language::*;
use async_trait::async_trait;
use tokio_stream::{Stream, StreamExt};
use futures::stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, error, debug, warn};

pub mod local;
pub mod remote;
pub mod config;

pub use local::{KalosmLocalModel, LlamaModel, MistralModel, PhiModel};
pub use remote::{OpenAIModel, AnthropicModel, GeminiModel, OllamaModel};
pub use config::ModelConfig;

/// Enhanced Model Manager with support for multiple AI providers
pub struct ModelManager {
    sse_manager: Arc<RwLock<SSEModelManager>>,
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
        let sse_manager = Arc::new(RwLock::new(SSEModelManager::new()));
        
        let manager = Self {
            sse_manager,
            config,
            active_models: Arc::new(RwLock::new(HashMap::new())),
            model_metrics: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize models based on config
        manager.initialize_models().await?;
        
        Ok(manager)
    }
    
    async fn initialize_models(&self) -> Result<()> {
        info!("Initializing AI models...");
        
        // Initialize local models (Kalosm)
        if self.config.enable_local_models {
            self.init_local_models().await?;
        }
        
        // Initialize remote models
        if !self.config.openai_api_key.is_empty() {
            self.init_openai_models().await?;
        }
        
        if !self.config.anthropic_api_key.is_empty() {
            self.init_anthropic_models().await?;
        }
        
        if !self.config.gemini_api_key.is_empty() {
            self.init_gemini_models().await?;
        }
        
        if !self.config.ollama_endpoint.is_empty() {
            self.init_ollama_models().await?;
        }
        
        info!("Model initialization complete");
        Ok(())
    }
    
    async fn init_local_models(&self) -> Result<()> {
        info!("Initializing local Kalosm models...");
        
        // Llama models
        if self.config.enable_llama {
            let llama_model = Arc::new(LlamaModel::new().await?);
            self.register_model(llama_model).await;
        }
        
        // Mistral models
        if self.config.enable_mistral {
            let mistral_model = Arc::new(MistralModel::new().await?);
            self.register_model(mistral_model).await;
        }
        
        // Phi models
        if self.config.enable_phi {
            let phi_model = Arc::new(PhiModel::new().await?);
            self.register_model(phi_model).await;
        }
        
        Ok(())
    }
    
    async fn init_openai_models(&self) -> Result<()> {
        info!("Initializing OpenAI models...");
        
        let api_key = self.config.openai_api_key.clone();
        
        // GPT-4o
        let gpt4o = Arc::new(OpenAIModel::new("gpt-4o".to_string(), api_key.clone()).await?);
        self.register_model(gpt4o).await;
        
        // GPT-4o-mini
        let gpt4o_mini = Arc::new(OpenAIModel::new("gpt-4o-mini".to_string(), api_key.clone()).await?);
        self.register_model(gpt4o_mini).await;
        
        Ok(())
    }
    
    async fn init_anthropic_models(&self) -> Result<()> {
        info!("Initializing Anthropic models...");
        
        let api_key = self.config.anthropic_api_key.clone();
        
        // Claude 3.5 Sonnet
        let claude_sonnet = Arc::new(AnthropicModel::new("claude-3-5-sonnet-20241022".to_string(), api_key.clone()).await?);
        self.register_model(claude_sonnet).await;
        
        // Claude 3.5 Haiku
        let claude_haiku = Arc::new(AnthropicModel::new("claude-3-5-haiku-20241022".to_string(), api_key.clone()).await?);
        self.register_model(claude_haiku).await;
        
        Ok(())
    }
    
    async fn init_gemini_models(&self) -> Result<()> {
        info!("Initializing Gemini models...");
        
        let api_key = self.config.gemini_api_key.clone();
        
        // Gemini Pro
        let gemini_pro = Arc::new(GeminiModel::new("gemini-pro".to_string(), api_key).await?);
        self.register_model(gemini_pro).await;
        
        Ok(())
    }
    
    async fn init_ollama_models(&self) -> Result<()> {
        info!("Initializing Ollama models...");
        
        let endpoint = self.config.ollama_endpoint.clone();
        
        // Get available models from Ollama
        let ollama_model = OllamaModel::new(endpoint).await?;
        let available_models = ollama_model.list_models().await?;
        
        for model_name in available_models {
            let model = Arc::new(OllamaModel::new_with_model(
                self.config.ollama_endpoint.clone(),
                model_name,
            ).await?);
            self.register_model(model).await;
        }
        
        Ok(())
    }
    
    async fn register_model(&self, model: Arc<dyn AIModel + Send + Sync>) {
        let model_id = model.id();
        info!("Registering model: {}", model_id);
        
        // Add to active models
        {
            let mut models = self.active_models.write().await;
            models.insert(model_id.clone(), model.clone());
        }
        
        // Initialize metrics
        {
            let mut metrics = self.model_metrics.write().await;
            metrics.insert(model_id.clone(), ModelMetrics::default());
        }
        
        // Register with SSE manager
        {
            let mut sse_manager = self.sse_manager.write().await;
            match model.model_type() {
                ModelType::Local => {
                    let local_model = KalosmLocalModel::new(model);
                    sse_manager.add_local_model(model_id, Box::new(local_model)).await;
                }
                ModelType::Remote => {
                    let remote_model = RemoteModelAdapter::new(model);
                    sse_manager.add_remote_model(model_id, Box::new(remote_model)).await;
                }
            }
        }
    }
    
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        let models = self.active_models.read().await;
        let metrics = self.model_metrics.read().await;
        
        let mut model_infos = Vec::new();
        
        for (id, model) in models.iter() {
            let model_metrics = metrics.get(id);
            
            model_infos.push(ModelInfo {
                id: id.clone(),
                name: model.name(),
                model_type: model.model_type(),
                status: model.status().await,
                context_window: model.context_window(),
                supports_latex: model.supports_latex(),
                metrics: model_metrics.cloned(),
            });
        }
        
        model_infos
    }
    
    pub async fn get_model(&self, model_id: &str) -> Option<Arc<dyn AIModel + Send + Sync>> {
        let models = self.active_models.read().await;
        models.get(model_id).cloned()
    }
    
    pub async fn stream_response(&self, mut request: ModelRequest) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>> {
        let model_id = &request.model_id;
        
        let model = self.get_model(model_id).await
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;
        
        // Enhance request with LaTeX-specific context
        self.enhance_latex_context(&mut request).await;
        
        // Start timing for metrics
        let start_time = std::time::Instant::now();
        
        // Stream the response
        let response_stream = model.stream_response(request).await?;
        
        // Wrap stream to collect metrics
        let model_id_clone = model_id.clone();
        let metrics = Arc::clone(&self.model_metrics);
        
        let wrapped_stream = response_stream.map(move |response| {
            // Update metrics
            if let Some(tokens) = response.tokens_used {
                tokio::spawn({
                    let model_id = model_id_clone.clone();
                    let metrics = Arc::clone(&metrics);
                    async move {
                        let mut metrics_guard = metrics.write().await;
                        if let Some(model_metrics) = metrics_guard.get_mut(&model_id) {
                            model_metrics.total_requests += 1;
                            model_metrics.total_tokens += tokens as u64;
                            model_metrics.last_used = Some(chrono::Utc::now());
                            
                            // Calculate tokens per second
                            let elapsed = start_time.elapsed().as_secs_f32();
                            if elapsed > 0.0 {
                                model_metrics.average_tokens_per_second = tokens as f32 / elapsed;
                            }
                        }
                    }
                });
            }
            
            response
        });
        
        Ok(Box::new(wrapped_stream))
    }
    
    async fn enhance_latex_context(&self, request: &mut ModelRequest) {
        // Add LaTeX-specific system prompts and context
        if let Some(context) = &mut request.context {
            // Add LaTeX expertise prompt
            let latex_prompt = r#"
You are an expert LaTeX assistant. You understand LaTeX syntax, commands, packages, and best practices. 
When generating LaTeX code:
1. Use proper LaTeX syntax and commands
2. Suggest appropriate packages when needed
3. Follow LaTeX best practices for document structure
4. Explain complex LaTeX concepts clearly
5. Help with mathematical typesetting, figures, tables, and bibliographies
6. Consider document class and existing packages in context
"#;
            
            request.prompt = format!("{}\n\n{}", latex_prompt, request.prompt);
            
            // Add document structure context
            if !context.packages.is_empty() {
                let packages_info = format!("Current document uses packages: {}", context.packages.join(", "));
                request.prompt = format!("{}\n\n{}", request.prompt, packages_info);
            }
            
            if let Some(doc_class) = &context.document_class {
                let class_info = format!("Document class: {}", doc_class);
                request.prompt = format!("{}\n\n{}", request.prompt, class_info);
            }
        }
    }
    
    pub async fn get_model_metrics(&self, model_id: &str) -> Option<ModelMetrics> {
        let metrics = self.model_metrics.read().await;
        metrics.get(model_id).cloned()
    }
    
    pub async fn get_sse_manager(&self) -> Arc<RwLock<SSEModelManager>> {
        Arc::clone(&self.sse_manager)
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
    pub metrics: Option<ModelMetrics>,
}

// Adapter for wrapping AIModel as LocalModel
pub struct KalosmLocalModel {
    model: Arc<dyn AIModel + Send + Sync>,
}

impl KalosmLocalModel {
    pub fn new(model: Arc<dyn AIModel + Send + Sync>) -> Self {
        Self { model }
    }
}

#[async_trait]
impl LocalModel for KalosmLocalModel {
    fn name(&self) -> String {
        self.model.name()
    }
    
    async fn status(&self) -> ModelStatus {
        self.model.status().await
    }
    
    async fn stream_response(&self, request: ModelRequest) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>> {
        self.model.stream_response(request).await
    }
}

// Adapter for wrapping AIModel as RemoteModel
pub struct RemoteModelAdapter {
    model: Arc<dyn AIModel + Send + Sync>,
}

impl RemoteModelAdapter {
    pub fn new(model: Arc<dyn AIModel + Send + Sync>) -> Self {
        Self { model }
    }
}

#[async_trait]
impl RemoteModel for RemoteModelAdapter {
    fn name(&self) -> String {
        self.model.name()
    }
    
    async fn status(&self) -> ModelStatus {
        self.model.status().await
    }
    
    async fn stream_response(&self, request: ModelRequest) -> Result<Box<dyn Stream<Item = ModelResponse> + Send + Unpin>> {
        self.model.stream_response(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_model_manager_creation() {
        let config = ModelConfig::default();
        let manager = ModelManager::new(config).await;
        assert!(manager.is_ok());
    }
}