use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub workspace_path: PathBuf,
    pub server: ServerConfig,
    pub agent: AgentConfig,
    pub clerk_secret_key: Option<String>,
    pub tavily_api_key: Option<String>,
    pub polar_access_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub litellm_base_url: String,
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
        
        let litellm_base_url = std::env::var("LITELLM_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4000".to_string());

        let clerk_secret_key = std::env::var("CLERK_SECRET_KEY").ok();
        let tavily_api_key = std::env::var("TAVILY_API_KEY").ok();
        let polar_access_token = std::env::var("POLAR_ACCESS_TOKEN").ok();

        Ok(Config {
            workspace_path,
            server: ServerConfig { host, port },
            agent: AgentConfig { litellm_base_url },
            clerk_secret_key,
            tavily_api_key,
            polar_access_token,
        })
    }
}