use std::time::Duration;
use anyhow::Result;
use reqwest::Client;
use tracing::{debug, error, warn};

pub mod search;
pub mod extract;

pub use search::{SearchRequest, SearchResponse, SearchResult};
pub use extract::{ExtractRequest, ExtractResponse, ExtractResult};

/// Main Tavily repository for web search and content extraction
pub struct TavilyRepository {
    client: Client,
    base_url: String,
    api_key: String,
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
    /// Creates a new Tavily repository instance with API key
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow::anyhow!("Tavily API key is required").into());
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()?;

        // Get base URL from environment, default to direct API URL
        let base_url = std::env::var("TAVILY_BASE_URL")
            .unwrap_or_else(|_| "https://api.tavily.com".to_string());

        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }
    
    /// Performs a web search using Tavily's search API
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse, TavilyError> {
        debug!("Performing Tavily search for query: {}", request.query);
        
        let url = format!("{}/search", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
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
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
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

