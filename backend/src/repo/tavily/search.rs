use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request structure for Tavily search API
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    /// The search query to execute
    pub query: String,
    
    /// Search category: "general", "news", "finance"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    
    /// Search depth: "basic" or "advanced"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_depth: Option<String>,
    
    /// Maximum number of results (default 5, max 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    
    /// Generate an answer to the query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_answer: Option<bool>,
    
    /// Include webpage content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_raw_content: Option<bool>,
    
    /// Perform image search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            topic: None,
            search_depth: Some("basic".to_string()),
            max_results: Some(5),
            include_answer: Some(false),
            include_raw_content: Some(false),
            include_images: Some(false),
        }
    }
}

/// Response structure from Tavily search API
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    /// Original search query
    pub query: String,
    
    /// LLM-generated answer (if requested)
    #[serde(default)]
    pub answer: Option<String>,
    
    /// Array of search results
    #[serde(default)]
    pub results: Vec<SearchResult>,
    
    /// Related images
    #[serde(default)]
    pub images: Vec<String>,
    
    /// Request processing time
    #[serde(default)]
    pub response_time: Option<f64>,
    
    /// Unique request identifier
    #[serde(default)]
    pub request_id: Option<String>,
    
    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual search result from Tavily
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult {
    /// Result title
    pub title: String,
    
    /// Source URL
    pub url: String,
    
    /// Result description/content
    pub content: String,
    
    /// Relevance score
    #[serde(default)]
    pub score: Option<f64>,
    
    /// Raw content (if requested)
    #[serde(default)]
    pub raw_content: Option<String>,
}

impl SearchRequest {
    /// Creates a new search request with just a query
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }
    
    /// Sets the search topic
    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }
    
    /// Sets the search depth
    pub fn with_depth(mut self, depth: impl Into<String>) -> Self {
        self.search_depth = Some(depth.into());
        self
    }
    
    /// Sets the maximum number of results
    pub fn with_max_results(mut self, max_results: u32) -> Self {
        self.max_results = Some(max_results.min(20)); // API limit is 20
        self
    }
    
    /// Enables answer generation
    pub fn with_answer(mut self, include: bool) -> Self {
        self.include_answer = Some(include);
        self
    }
    
    /// Enables raw content inclusion
    pub fn with_raw_content(mut self, include: bool) -> Self {
        self.include_raw_content = Some(include);
        self
    }
    
    /// Enables image search
    pub fn with_images(mut self, include: bool) -> Self {
        self.include_images = Some(include);
        self
    }
}

impl SearchResponse {
    /// Returns the total number of results
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
    
    /// Returns results with score above threshold
    pub fn high_quality_results(&self, min_score: f64) -> Vec<&SearchResult> {
        self.results
            .iter()
            .filter(|result| result.score.unwrap_or(0.0) >= min_score)
            .collect()
    }
    
    /// Returns all URLs from the results
    pub fn urls(&self) -> Vec<&str> {
        self.results.iter().map(|r| r.url.as_str()).collect()
    }
    
    /// Returns a summary of the search results
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

