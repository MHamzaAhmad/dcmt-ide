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
    #[allow(dead_code)]
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
    
    async fn enhance_latex_context(&self, request: &mut ModelRequest) {
        // Enhance LaTeX context for AI models
        if let Some(context) = &mut request.context {
            // Analyze existing LaTeX context
            let doc_class = detect_document_class(&context.document_content).unwrap_or("article".to_string());
            let packages = detect_packages(&context.document_content);
            let current_section = detect_current_section(&context.document_content).unwrap_or("Unknown".to_string());
            let math_mode_active = context.document_content.contains("$") || context.document_content.contains("\\[");
            
            // Update packages list and document class in context
            context.packages = packages.clone();
            context.document_class = Some(doc_class.clone());
            
            // Enhance document content with analysis
            let enhanced_content = format!(
                r#"LaTeX Document Analysis:
- Document class: {}
- Packages in use: {}
- Current section: {}
- Math mode: {}

Instructions: You are helping with LaTeX document editing. 
Provide suggestions that are syntactically correct and follow LaTeX best practices.
When suggesting code, use proper LaTeX commands and environments.

Document content:
{}"#,
                doc_class,
                packages.join(", "),
                current_section,
                if math_mode_active { "Active" } else { "Inactive" },
                context.document_content
            );
            
            context.document_content = enhanced_content;
        } else {
            // Create default LaTeX context if none exists
            use latex_ide_sse_handler::LaTeXContext;
            request.context = Some(LaTeXContext {
                document_content: "LaTeX Document Context: You are helping with LaTeX document editing. Provide suggestions that are syntactically correct and follow LaTeX best practices.".to_string(),
                current_position: None,
                selected_text: None,
                packages: vec![],
                document_class: Some("article".to_string()),
            });
        }
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

// Helper functions for LaTeX context analysis
fn detect_document_class(latex: &str) -> Option<String> {
    let re = regex::Regex::new(r"\\documentclass(?:\[[^\]]*\])?\{([^}]+)\}").ok()?;
    re.captures(latex)?.get(1).map(|m| m.as_str().to_string())
}

fn detect_packages(latex: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\\usepackage(?:\[[^\]]*\])?\{([^}]+)\}").unwrap();
    re.captures_iter(latex)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

fn detect_current_section(latex: &str) -> Option<String> {
    let patterns = [
        r"\\chapter\{([^}]+)\}",
        r"\\section\{([^}]+)\}",
        r"\\subsection\{([^}]+)\}",
        r"\\subsubsection\{([^}]+)\}",
    ];
    
    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(last_match) = re.captures_iter(latex).last() {
                if let Some(title) = last_match.get(1) {
                    return Some(title.as_str().to_string());
                }
            }
        }
    }
    None
}