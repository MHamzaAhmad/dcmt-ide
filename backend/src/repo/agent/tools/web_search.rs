use super::*;
use crate::repo::tavily::{TavilyRepository, SearchRequest};
use std::path::Path;

/// Tool for performing web searches using Tavily API
pub struct WebSearchTool;

#[async_trait]
impl AgentTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }
    
    fn display_name(&self) -> &str {
        "Web Search"
    }
    
    fn progressive_form(&self) -> &str {
        "Searching the web"
    }
    
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: "Search the web for current information, news, research papers, documentation, or any topic. Returns relevant search results with titles, URLs, content snippets, and relevance scores.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query to execute (e.g., 'latest developments in AI', 'LaTeX documentation', 'how to debug Rust errors')"
                        },
                        "topic": {
                            "type": "string",
                            "enum": ["general", "news", "finance"],
                            "description": "Search category to focus results (optional)"
                        },
                        "max_results": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 20,
                            "default": 5,
                            "description": "Maximum number of search results to return"
                        },
                        "include_answer": {
                            "type": "boolean",
                            "default": false,
                            "description": "Generate an AI-powered answer to the query"
                        },
                        "include_images": {
                            "type": "boolean", 
                            "default": false,
                            "description": "Include related images in search results"
                        },
                        "search_depth": {
                            "type": "string",
                            "enum": ["basic", "advanced"],
                            "default": "basic",
                            "description": "Search depth - advanced provides more comprehensive results"
                        }
                    },
                    "required": ["query"]
                }),
                display_name: Some(self.display_name().to_string()),
                progressive_form: Some(self.progressive_form().to_string()),
            },
        }
    }
    
    async fn execute(&self, _workspace: &Path, args: Value, _repo: Option<&crate::repo::agent::AgentRepo>) -> AgentResult<String> {
        // Extract required parameters
        let query = args["query"]
            .as_str()
            .ok_or_else(|| AgentError::InvalidToolArguments { 
                tool: self.name().to_string(), 
                error: "Missing 'query' parameter".to_string() 
            })?;

        if query.trim().is_empty() {
            return Err(AgentError::InvalidToolArguments {
                tool: self.name().to_string(),
                error: "Query cannot be empty".to_string(),
            });
        }
        
        // Build search request
        let mut request = SearchRequest::new(query);
        
        // Add optional parameters
        if let Some(topic) = args["topic"].as_str() {
            request = request.with_topic(topic);
        }
        
        if let Some(max_results) = args["max_results"].as_u64() {
            request = request.with_max_results(max_results as u32);
        }
        
        if let Some(include_answer) = args["include_answer"].as_bool() {
            request = request.with_answer(include_answer);
        }
        
        if let Some(include_images) = args["include_images"].as_bool() {
            request = request.with_images(include_images);
        }
        
        if let Some(search_depth) = args["search_depth"].as_str() {
            request = request.with_depth(search_depth);
        }
        
        // Create Tavily repository and perform search
        let tavily_repo = TavilyRepository::new()
            .map_err(|e| AgentError::ToolExecutionError {
                tool: self.name().to_string(),
                error: format!("Failed to create Tavily client: {}", e),
            })?;
            
        tracing::info!("Performing web search for query: {}", query);
        
        let response = tavily_repo.search(request).await
            .map_err(|e| AgentError::ToolExecutionError {
                tool: self.name().to_string(),
                error: format!("Web search failed: {}", e),
            })?;
            
        // Format the response
        let mut result = String::new();
        
        // Add generated answer if available
        if let Some(answer) = &response.answer {
            result.push_str(&format!("## AI-Generated Answer\n{}\n\n", answer));
        }
        
        // Add search results
        if !response.results.is_empty() {
            result.push_str(&format!("## Search Results ({} found)\n\n", response.results.len()));
            
            for (i, search_result) in response.results.iter().enumerate() {
                result.push_str(&format!(
                    "### {}. {}\n**URL:** {}\n**Content:** {}\n",
                    i + 1,
                    search_result.title,
                    search_result.url,
                    search_result.content
                ));
                
                if let Some(score) = search_result.score {
                    result.push_str(&format!("**Relevance Score:** {:.2}\n", score));
                }
                
                result.push('\n');
            }
        }
        
        // Add images if requested and available
        if !response.images.is_empty() {
            result.push_str("## Related Images\n");
            for (i, image_url) in response.images.iter().enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, image_url));
            }
            result.push('\n');
        }
        
        // Add metadata
        if let Some(response_time) = response.response_time {
            result.push_str(&format!("*Search completed in {:.2}s*\n", response_time));
        }
        
        if result.is_empty() {
            result = format!("No results found for query: {}", query);
        }
        
        tracing::info!("Web search completed successfully with {} results", response.results.len());
        Ok(result)
    }
}

