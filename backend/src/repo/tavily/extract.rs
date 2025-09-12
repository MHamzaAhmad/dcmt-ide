use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request structure for Tavily extract API
#[derive(Debug, Clone, Serialize)]
pub struct ExtractRequest {
    /// Single URL or array of URLs to extract content from
    pub urls: Vec<String>,
    
    /// Include image URLs in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_images: Option<bool>,
    
    /// Include favicon URL in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_favicon: Option<bool>,
    
    /// Extraction depth: "basic" or "advanced"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_depth: Option<String>,
    
    /// Output format: "markdown" or "text"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    
    /// Timeout in seconds (1.0-60.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f32>,
}

impl Default for ExtractRequest {
    fn default() -> Self {
        Self {
            urls: Vec::new(),
            include_images: Some(false),
            include_favicon: Some(false),
            extract_depth: Some("basic".to_string()),
            format: Some("markdown".to_string()),
            timeout: Some(15.0),
        }
    }
}

/// Response structure from Tavily extract API
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractResponse {
    /// Array of extracted content objects
    #[serde(default)]
    pub results: Vec<ExtractResult>,
    
    /// List of URLs that couldn't be processed
    #[serde(default)]
    pub failed_results: Vec<FailedResult>,
    
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

/// Individual extraction result from Tavily
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractResult {
    /// Source URL
    pub url: String,
    
    /// Extracted page content
    pub raw_content: String,
    
    /// Optional image URLs
    #[serde(default)]
    pub images: Option<Vec<String>>,
    
    /// Optional favicon URL
    #[serde(default)]
    pub favicon: Option<String>,
    
    /// Page title (if available)
    #[serde(default)]
    pub title: Option<String>,
    
    /// Content length in characters
    #[serde(default)]
    pub content_length: Option<usize>,
    
    /// Additional metadata about the extraction
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Failed extraction result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FailedResult {
    /// URL that failed to be processed
    pub url: String,
    
    /// Error message
    pub error: String,
    
    /// Error code (if available)
    #[serde(default)]
    pub error_code: Option<String>,
}

impl ExtractRequest {
    /// Creates a new extract request with a single URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            urls: vec![url.into()],
            ..Default::default()
        }
    }
    
    /// Creates a new extract request with multiple URLs
    pub fn new_batch(urls: Vec<String>) -> Self {
        Self {
            urls,
            ..Default::default()
        }
    }
    
    /// Adds a URL to the extraction request
    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }
    
    /// Sets whether to include images
    pub fn with_images(mut self, include: bool) -> Self {
        self.include_images = Some(include);
        self
    }
    
    /// Sets whether to include favicon
    pub fn with_favicon(mut self, include: bool) -> Self {
        self.include_favicon = Some(include);
        self
    }
    
    /// Sets the extraction depth
    pub fn with_depth(mut self, depth: impl Into<String>) -> Self {
        self.extract_depth = Some(depth.into());
        self
    }
    
    /// Sets the output format
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
    
    /// Sets the timeout (clamped to 1.0-60.0 range)
    pub fn with_timeout(mut self, timeout: f32) -> Self {
        self.timeout = Some(timeout.max(1.0).min(60.0));
        self
    }
    
    /// Validates the request
    pub fn is_valid(&self) -> bool {
        !self.urls.is_empty() && 
        self.urls.iter().all(|url| !url.is_empty()) &&
        self.timeout.unwrap_or(15.0) >= 1.0 && 
        self.timeout.unwrap_or(15.0) <= 60.0
    }
}

impl ExtractResponse {
    /// Returns the total number of successful extractions
    pub fn success_count(&self) -> usize {
        self.results.len()
    }
    
    /// Returns the total number of failed extractions
    pub fn failure_count(&self) -> usize {
        self.failed_results.len()
    }
    
    /// Returns the total number of URLs processed
    pub fn total_count(&self) -> usize {
        self.success_count() + self.failure_count()
    }
    
    /// Returns success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_count() == 0 {
            0.0
        } else {
            (self.success_count() as f64 / self.total_count() as f64) * 100.0
        }
    }
    
    /// Returns all successfully extracted content
    pub fn get_all_content(&self) -> Vec<String> {
        self.results.iter().map(|r| r.raw_content.clone()).collect()
    }
    
    /// Returns content from a specific URL
    pub fn get_content_for_url(&self, url: &str) -> Option<&str> {
        self.results
            .iter()
            .find(|r| r.url == url)
            .map(|r| r.raw_content.as_str())
    }
    
    /// Returns all failed URLs with their error messages
    pub fn get_failed_urls(&self) -> Vec<(&str, &str)> {
        self.failed_results
            .iter()
            .map(|f| (f.url.as_str(), f.error.as_str()))
            .collect()
    }
    
    /// Returns a summary of the extraction results
    pub fn summary(&self) -> String {
        match (self.success_count(), self.failure_count()) {
            (0, 0) => "No URLs were processed".to_string(),
            (s, 0) => format!("Successfully extracted content from {} URL(s)", s),
            (0, f) => format!("Failed to extract content from {} URL(s)", f),
            (s, f) => format!("Extracted content from {} URL(s), {} failed", s, f),
        }
    }
}

impl ExtractResult {
    /// Returns the content length
    pub fn len(&self) -> usize {
        self.content_length.unwrap_or_else(|| self.raw_content.len())
    }
    
    /// Checks if the extraction result is empty
    pub fn is_empty(&self) -> bool {
        self.raw_content.is_empty()
    }
    
    /// Returns a truncated version of the content
    pub fn preview(&self, max_chars: usize) -> String {
        if self.raw_content.len() <= max_chars {
            self.raw_content.clone()
        } else {
            format!("{}...", &self.raw_content[..max_chars])
        }
    }
}

