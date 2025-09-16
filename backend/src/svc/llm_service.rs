use anyhow::Result;
use tracing::{debug, error};
use crate::repo::llm::{LLMRepository, LLMModelsResponse};

/// Service for LLM operations
pub struct LLMService {
    repository: LLMRepository,
}

impl LLMService {
    /// Creates a new LLM service instance
    pub fn new(base_url: String) -> Result<Self> {
        let repository = LLMRepository::new(base_url)?;
        Ok(Self { repository })
    }

    /// Lists available models from the LLM service
    pub async fn list_models(&self) -> Result<LLMModelsResponse> {
        debug!("LLM service: listing models");

        match self.repository.list_models().await {
            Ok(models) => {
                debug!("LLM service: successfully retrieved {} models", models.data.len());
                Ok(models)
            }
            Err(e) => {
                error!("LLM service: failed to list models - {}", e);
                Err(e.into())
            }
        }
    }
}