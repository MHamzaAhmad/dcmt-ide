use std::path::PathBuf;
use anyhow::Result;
use crate::model::agent::AgentConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub workspace_path: PathBuf,
    pub server: ServerConfig,
    pub agent: AgentConfig,
    pub clerk_secret_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Load .env file if it exists (ignore errors if file doesn't exist)
        let _ = dotenvy::dotenv();
        
        let workspace_path = std::env::var("DCMT_WORKSPACE_PATH")
            .unwrap_or_else(|_| "./workspace".to_string())
            .into();
        
        let host = std::env::var("DCMT_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        
        let port = std::env::var("DCMT_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()
            .map_err(|e| anyhow::anyhow!("Invalid DCMT_PORT value: {}", e))?;
        
  
        let tavily_api_key = std::env::var("TAVILY_API_KEY").ok();
        let clerk_secret_key = std::env::var("CLERK_SECRET_KEY").ok();

        // Load system prompt (placeholder for now)
        let system_prompt = std::env::var("SYSTEM_PROMPT")
            .unwrap_or_else(|_| "You are a helpful AI assistant.".to_string());

        Ok(Config {
            workspace_path,
            server: ServerConfig { host, port },
            agent: AgentConfig {
                system_prompt,
                max_session_age: std::time::Duration::from_secs(3600),
                tavily_api_key: tavily_api_key.clone(),
            },
            clerk_secret_key,
        })
    }
}