use std::time::Duration;
use anyhow::Result;
use reqwest::Client;
use serde_json;
use tracing::{debug, error, warn};

pub mod search;
pub mod extract;

pub use search::{SearchRequest, SearchResponse, SearchResult};
pub use extract::{ExtractRequest, ExtractResponse, ExtractResult};

/// Main Tavily repository for web search and content extraction
pub struct TavilyRepository {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TavilyError {
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
}

impl TavilyRepository {
    /// Creates a new Tavily repository instance using direct API
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url: "https://api.tavily.com".to_string(),
        })
    }
    
    /// Performs a web search using Tavily's search API
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse, TavilyError> {
        debug!("Performing Tavily search for query: {}", request.query);

        let url = format!("{}/search", self.base_url);

        let mut api_request = serde_json::Map::new();
        api_request.insert("api_key".to_string(), serde_json::Value::String(self.api_key.clone()));
        api_request.insert("query".to_string(), serde_json::Value::String(request.query.clone()));

        if let Some(topic) = &request.topic {
            api_request.insert("topic".to_string(), serde_json::Value::String(topic.clone()));
        }
        if let Some(search_depth) = &request.search_depth {
            api_request.insert("search_depth".to_string(), serde_json::Value::String(search_depth.clone()));
        }
        if let Some(max_results) = &request.max_results {
            api_request.insert("max_results".to_string(), serde_json::Value::Number(serde_json::Number::from(*max_results)));
        }
        if let Some(include_answer) = &request.include_answer {
            api_request.insert("include_answer".to_string(), serde_json::Value::Bool(*include_answer));
        }
        if let Some(include_raw_content) = &request.include_raw_content {
            api_request.insert("include_raw_content".to_string(), serde_json::Value::Bool(*include_raw_content));
        }
        if let Some(include_images) = &request.include_images {
            api_request.insert("include_images".to_string(), serde_json::Value::Bool(*include_images));
        }

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }
        
        let search_response: SearchResponse = response.json().await
            .map_err(|e| TavilyError::InvalidResponse(e.to_string()))?;
            
        debug!("Search completed successfully with {} results", search_response.results.len());
        Ok(search_response)
    }
    
    /// Extracts content from URLs using Tavily's extract API
    pub async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse, TavilyError> {
        debug!("Extracting content from {} URLs", request.urls.len());

        let url = format!("{}/extract", self.base_url);

        let mut api_request = serde_json::Map::new();
        api_request.insert("api_key".to_string(), serde_json::Value::String(self.api_key.clone()));
        api_request.insert("urls".to_string(), serde_json::Value::Array(
            request.urls.iter().map(|url| serde_json::Value::String(url.clone())).collect()
        ));

        if let Some(include_images) = &request.include_images {
            api_request.insert("include_images".to_string(), serde_json::Value::Bool(*include_images));
        }
        if let Some(include_favicon) = &request.include_favicon {
            api_request.insert("include_favicon".to_string(), serde_json::Value::Bool(*include_favicon));
        }
        if let Some(extract_depth) = &request.extract_depth {
            api_request.insert("extract_depth".to_string(), serde_json::Value::String(extract_depth.clone()));
        }
        if let Some(format) = &request.format {
            api_request.insert("format".to_string(), serde_json::Value::String(format.clone()));
        }
        if let Some(timeout) = &request.timeout {
            if let Some(timeout_num) = serde_json::Number::from_f64(*timeout as f64) {
                api_request.insert("timeout".to_string(), serde_json::Value::Number(timeout_num));
            }
        }

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return self.handle_error_response(response).await;
        }
        
        let extract_response: ExtractResponse = response.json().await
            .map_err(|e| TavilyError::InvalidResponse(e.to_string()))?;
            
        debug!("Content extraction completed successfully for {} URLs", 
               extract_response.results.len());
        Ok(extract_response)
    }
    
    /// Handles error responses from the Tavily API
    async fn handle_error_response<T>(&self, response: reqwest::Response) -> Result<T, TavilyError> {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        
        match status.as_u16() {
            401 => {
                error!("Tavily API authentication failed");
                Err(TavilyError::InvalidApiKey)
            }
            429 => {
                warn!("Tavily API rate limit exceeded");
                Err(TavilyError::RateLimit)
            }
            408 | 504 => {
                warn!("Tavily API request timeout");
                Err(TavilyError::Timeout)
            }
            _ => {
                error!("Tavily API error {}: {}", status, error_text);
                Err(TavilyError::Api {
                    message: format!("HTTP {}: {}", status, error_text)
                })
            }
        }
    }
}

