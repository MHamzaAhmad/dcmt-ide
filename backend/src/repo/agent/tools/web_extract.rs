use super::*;
use crate::repo::tavily::{TavilyRepository, ExtractRequest};
use std::path::Path;

/// Tool for extracting content from web URLs using Tavily API
pub struct WebExtractTool;

#[async_trait]
impl AgentTool for WebExtractTool {
    fn name(&self) -> &str {
        "web_extract"
    }
    
    fn display_name(&self) -> &str {
        "Extract Web Content"
    }
    
    fn progressive_form(&self) -> &str {
        "Extracting web content"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Extract and parse content from web pages, articles, documentation, or any URL. Returns clean, readable text content from the specified URLs, perfect for analysis or summarization.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "urls": {
                            "oneOf": [
                                {
                                    "type": "string",
                                    "description": "A single URL to extract content from"
                                },
                                {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "Multiple URLs to extract content from (processed in parallel)"
                                }
                            ],
                            "description": "URL(s) to extract content from (e.g., 'https://example.com/article' or ['https://site1.com', 'https://site2.com'])"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["markdown", "text"],
                            "default": "markdown",
                            "description": "Output format for extracted content"
                        },
                        "include_images": {
                            "type": "boolean",
                            "default": false,
                            "description": "Include image URLs found on the page"
                        },
                        "include_favicon": {
                            "type": "boolean",
                            "default": false,
                            "description": "Include the website's favicon URL"
                        },
                        "extract_depth": {
                            "type": "string",
                            "enum": ["basic", "advanced"],
                            "default": "basic",
                            "description": "Extraction depth - advanced provides more comprehensive content parsing"
                        },
                        "timeout": {
                            "type": "number",
                            "minimum": 1.0,
                            "maximum": 60.0,
                            "default": 15.0,
                            "description": "Timeout in seconds for each URL extraction"
                        }
                    },
                    "required": ["urls"]
                }),
                display_name: Some(self.display_name().to_string()),
                progressive_form: Some(self.progressive_form().to_string()),
            },
        }
    }
    
    async fn execute(&self, _workspace: &Path, args: Value, _repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String> {
        // Extract URLs parameter (can be string or array)
        let urls = match &args["urls"] {
            Value::String(url) => vec![url.clone()],
            Value::Array(url_array) => {
                url_array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }
            _ => {
                return Err(AgentError::InvalidToolArguments {
                    tool: self.name().to_string(),
                    error: "Missing or invalid 'urls' parameter - must be string or array of strings".to_string(),
                });
            }
        };

        if urls.is_empty() {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "At least one URL must be provided".to_string(),
            });
        }

        // Validate URLs
        for url in &urls {
            if url.trim().is_empty() {
                return Err(AgentError::InvalidToolArguments {
                    tool: self.name().to_string(),
                    error: "URLs cannot be empty".to_string(),
                });
            }
            
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(AgentError::InvalidToolArguments {
                    tool: self.name().to_string(),
                    error: format!("Invalid URL format: {}. URLs must start with http:// or https://", url),
                });
            }
        }
        
        // Build extract request
        let mut request = ExtractRequest::new_batch(urls.clone());
        
        // Add optional parameters
        if let Some(format) = args["format"].as_str() {
            request = request.with_format(format);
        }
        
        if let Some(include_images) = args["include_images"].as_bool() {
            request = request.with_images(include_images);
        }
        
        if let Some(include_favicon) = args["include_favicon"].as_bool() {
            request = request.with_favicon(include_favicon);
        }
        
        if let Some(extract_depth) = args["extract_depth"].as_str() {
            request = request.with_depth(extract_depth);
        }
        
        if let Some(timeout) = args["timeout"].as_f64() {
            request = request.with_timeout(timeout as f32);
        }
        
        if !request.is_valid() {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Invalid extraction request parameters".to_string(),
            });
        }
        
        // Get Tavily API key from environment
        let api_key = std::env::var("TAVILY_API_KEY")
            .map_err(|_| AgentError::Generic(anyhow::anyhow!(
                "TAVILY_API_KEY environment variable not set"
            )))?;
            
        // Create Tavily repository and perform extraction
        let tavily_repo = TavilyRepository::new(api_key)
            .map_err(|e| AgentError::ToolExecutionError {
                tool: self.name().to_string(),
                error: format!("Failed to create Tavily client: {}", e),
            })?;
            
        tracing::info!("Extracting content from {} URL(s)", urls.len());
        
        let response = tavily_repo.extract(request).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: self.name().to_string(),
                error: format!("Content extraction failed: {}", e),
            })?;
            
        // Format the response
        let mut result = String::new();
        
        // Add summary
        result.push_str(&format!("## Content Extraction Summary\n{}\n\n", response.summary()));
        
        // Add successful extractions
        if !response.results.is_empty() {
            result.push_str(&format!("## Extracted Content ({} successful)\n\n", response.success_count()));
            
            for (i, extract_result) in response.results.iter().enumerate() {
                result.push_str(&format!(
                    "### {}. {}\n**Source:** {}\n**Content Length:** {} characters\n\n",
                    i + 1,
                    extract_result.title.as_deref().unwrap_or("Untitled"),
                    extract_result.url,
                    extract_result.len()
                ));
                
                // Add the actual content
                result.push_str("**Content:**\n");
                result.push_str(&extract_result.raw_content);
                result.push_str("\n\n");
                
                // Add images if available
                if let Some(images) = &extract_result.images {
                    if !images.is_empty() {
                        result.push_str("**Images:**\n");
                        for image_url in images {
                            result.push_str(&format!("- {}\n", image_url));
                        }
                        result.push('\n');
                    }
                }
                
                // Add favicon if available
                if let Some(favicon) = &extract_result.favicon {
                    result.push_str(&format!("**Favicon:** {}\n\n", favicon));
                }
                
                result.push_str("---\n\n");
            }
        }
        
        // Add failed extractions
        if !response.failed_results.is_empty() {
            result.push_str(&format!("## Failed Extractions ({} failed)\n\n", response.failure_count()));
            
            for (i, failed_result) in response.failed_results.iter().enumerate() {
                result.push_str(&format!(
                    "### {}. {}\n**Error:** {}\n\n",
                    i + 1,
                    failed_result.url,
                    failed_result.error
                ));
            }
        }
        
        // Add metadata
        if let Some(response_time) = response.response_time {
            result.push_str(&format!("*Extraction completed in {:.2}s*\n", response_time));
        }
        
        result.push_str(&format!("*Success Rate: {:.1}%*\n", response.success_rate()));
        
        if result.is_empty() {
            result = format!("Failed to extract content from all {} URL(s)", urls.len());
        }
        
        tracing::info!("Content extraction completed: {} successful, {} failed", 
                      response.success_count(), response.failure_count());
        Ok(result)
    }
}

