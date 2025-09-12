use std::time::Duration;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

/// Tavily search request structure for desktop service
#[derive(Debug, Clone, Serialize)]
pub struct TavilySearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_answer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_raw_content: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
}

/// Tavily extract request structure for desktop service
#[derive(Debug, Clone, Serialize)]
pub struct TavilyExtractRequest {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_favicon: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f32>,
}

/// Tavily search response structure
#[derive(Debug, Clone, Deserialize)]
pub struct TavilySearchResponse {
    pub query: String,
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub results: Vec<TavilySearchResult>,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub response_time: Option<f64>,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Individual search result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TavilySearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub raw_content: Option<String>,
}

/// Tavily extract response structure
#[derive(Debug, Clone, Deserialize)]
pub struct TavilyExtractResponse {
    #[serde(default)]
    pub results: Vec<TavilyExtractResult>,
    #[serde(default)]
    pub failed_results: Vec<TavilyFailedResult>,
    #[serde(default)]
    pub response_time: Option<f64>,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Individual extraction result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TavilyExtractResult {
    pub url: String,
    pub raw_content: String,
    #[serde(default)]
    pub images: Option<Vec<String>>,
    #[serde(default)]
    pub favicon: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content_length: Option<usize>,
}

/// Failed extraction result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TavilyFailedResult {
    pub url: String,
    pub error: String,
    #[serde(default)]
    pub error_code: Option<String>,
}

/// Error types for Tavily service
#[derive(Debug, thiserror::Error)]
pub enum TavilyServiceError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("API error: {message}")]
    Api { message: String },
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Request timeout")]
    Timeout,
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Main Tavily service for desktop application
pub struct TavilyService {
    client: Client,
    base_url: String,
}

impl TavilyService {
    /// Creates a new Tavily service instance using proxy
    pub fn new() -> Result<Self, TavilyServiceError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()?;
        
        // Get base URL from environment, default to local proxy
        let base_url = std::env::var("TAVILY_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8082".to_string());
            
        Ok(Self {
            client,
            base_url,
        })
    }
    
    /// Performs a web search using Tavily's search API
    pub async fn search(&self, request: TavilySearchRequest) -> Result<TavilySearchResponse, TavilyServiceError> {
        debug!("Performing Tavily search for query: {}", request.query);
        
        let url = format!("{}/search", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", "Bearer ")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }
        
        let search_response: TavilySearchResponse = response.json().await
            .map_err(|e| TavilyServiceError::InvalidResponse(e.to_string()))?;
            
        debug!("Search completed successfully with {} results", search_response.results.len());
        Ok(search_response)
    }
    
    /// Extracts content from URLs using Tavily's extract API
    pub async fn extract(&self, request: TavilyExtractRequest) -> Result<TavilyExtractResponse, TavilyServiceError> {
        debug!("Extracting content from {} URLs", request.urls.len());
        
        let url = format!("{}/extract", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", "Bearer ")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }
        
        let extract_response: TavilyExtractResponse = response.json().await
            .map_err(|e| TavilyServiceError::InvalidResponse(e.to_string()))?;
            
        debug!("Content extraction completed successfully for {} URLs", 
               extract_response.results.len());
        Ok(extract_response)
    }
    
    /// Handles error responses from the Tavily API
    async fn handle_error_response<T>(&self, response: reqwest::Response) -> Result<T, TavilyServiceError> {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        
        match status.as_u16() {
            401 => {
                error!("Tavily API authentication failed");
                Err(TavilyServiceError::InvalidApiKey)
            }
            429 => {
                warn!("Tavily API rate limit exceeded");
                Err(TavilyServiceError::RateLimit)
            }
            408 | 504 => {
                warn!("Tavily API request timeout");
                Err(TavilyServiceError::Timeout)
            }
            _ => {
                error!("Tavily API error {}: {}", status, error_text);
                Err(TavilyServiceError::Api {
                    message: format!("HTTP {}: {}", status, error_text)
                })
            }
        }
    }
}

// Convenience implementations for request builders
impl TavilySearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            topic: None,
            search_depth: Some("basic".to_string()),
            max_results: Some(5),
            include_answer: Some(false),
            include_raw_content: Some(false),
            include_images: Some(false),
        }
    }
    
    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }
    
    pub fn with_depth(mut self, depth: impl Into<String>) -> Self {
        self.search_depth = Some(depth.into());
        self
    }
    
    pub fn with_max_results(mut self, max_results: u32) -> Self {
        self.max_results = Some(max_results.min(20));
        self
    }
    
    pub fn with_answer(mut self, include: bool) -> Self {
        self.include_answer = Some(include);
        self
    }
    
    pub fn with_images(mut self, include: bool) -> Self {
        self.include_images = Some(include);
        self
    }
}

impl TavilyExtractRequest {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            urls: vec![url.into()],
            include_images: Some(false),
            include_favicon: Some(false),
            extract_depth: Some("basic".to_string()),
            format: Some("markdown".to_string()),
            timeout: Some(15.0),
        }
    }
    
    pub fn new_batch(urls: Vec<String>) -> Self {
        Self {
            urls,
            include_images: Some(false),
            include_favicon: Some(false),
            extract_depth: Some("basic".to_string()),
            format: Some("markdown".to_string()),
            timeout: Some(15.0),
        }
    }
    
    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }
    
    pub fn with_images(mut self, include: bool) -> Self {
        self.include_images = Some(include);
        self
    }
    
    pub fn with_favicon(mut self, include: bool) -> Self {
        self.include_favicon = Some(include);
        self
    }
    
    pub fn with_depth(mut self, depth: impl Into<String>) -> Self {
        self.extract_depth = Some(depth.into());
        self
    }
    
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
    
    pub fn with_timeout(mut self, timeout: f32) -> Self {
        self.timeout = Some(timeout.max(1.0).min(60.0));
        self
    }
}

// Utility methods for responses
impl TavilySearchResponse {
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
    
    pub fn summary(&self) -> String {
        if let Some(answer) = &self.answer {
            answer.clone()
        } else if !self.results.is_empty() {
            format!("Found {} results for query: {}", self.results.len(), self.query)
        } else {
            format!("No results found for query: {}", self.query)
        }
    }
}

impl TavilyExtractResponse {
    pub fn success_count(&self) -> usize {
        self.results.len()
    }
    
    pub fn failure_count(&self) -> usize {
        self.failed_results.len()
    }
    
    pub fn total_count(&self) -> usize {
        self.success_count() + self.failure_count()
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_count() == 0 {
            0.0
        } else {
            (self.success_count() as f64 / self.total_count() as f64) * 100.0
        }
    }
    
    pub fn summary(&self) -> String {
        match (self.success_count(), self.failure_count()) {
            (0, 0) => "No URLs were processed".to_string(),
            (s, 0) => format!("Successfully extracted content from {} URL(s)", s),
            (0, f) => format!("Failed to extract content from {} URL(s)", f),
            (s, f) => format!("Extracted content from {} URL(s), {} failed", s, f),
        }
    }
}

