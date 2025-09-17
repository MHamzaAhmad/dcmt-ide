use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;

use crate::repo::git_repository::{
    CommitResult, GitDiff, GitRepository, GitStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub summary: String,
    pub bullets: Vec<String>,
    #[serde(rename = "suggestedMessage")]
    pub suggested_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiteLLMRequest {
    model: String,
    messages: Vec<LiteLLMMessage>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiteLLMMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LiteLLMResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Choice {
    message: LiteLLMMessage,
}

pub struct GitService {
    repo: Option<Arc<GitRepository>>,
    workspace_path: PathBuf,
    http_client: Client,
    litellm_base_url: String,
}

impl GitService {
    pub fn new(workspace_path: PathBuf, litellm_base_url: String) -> Result<Self> {
        let repo = if GitRepository::repository_exists(&workspace_path) {
            Some(Arc::new(GitRepository::new(workspace_path.clone())?))
        } else {
            None
        };
        let http_client = Client::new();

        Ok(Self {
            repo,
            workspace_path,
            http_client,
            litellm_base_url,
        })
    }

    pub fn is_repository_initialized(&self) -> bool {
        self.repo.is_some()
    }

    pub async fn get_status(&self) -> Result<GitStatus> {
        match &self.repo {
            Some(repo) => repo.get_status(),
            None => Ok(GitStatus {
                branch: "main".to_string(),
                ahead: 0,
                behind: 0,
                staged: Vec::new(),
                unstaged: Vec::new(),
                untracked: Vec::new(),
                is_initialized: false,
                has_commits: false,
            }),
        }
    }

    pub async fn get_diff(&self, staged: bool) -> Result<GitDiff> {
        match &self.repo {
            Some(repo) => repo.get_diff(staged),
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn generate_commit_summary(&self, staged: bool) -> Result<CommitSummary> {
        match &self.repo {
            Some(repo) => {
                let diff_string = repo.get_diff_as_string(staged)?;

                if diff_string.is_empty() {
                    return Ok(CommitSummary {
                        summary: "No changes to commit".to_string(),
                        bullets: vec!["No changes detected".to_string()],
                        suggested_message: "chore: no changes".to_string(),
                    });
                }

        let prompt = self.load_prompt_template()?;
                let formatted_prompt = prompt.replace("{diff_content}", &diff_string);

                let request = LiteLLMRequest {
                    model: "gpt-4o-mini".to_string(),
                    messages: vec![
                        LiteLLMMessage {
                            role: "system".to_string(),
                            content: "You are a helpful assistant that generates git commit summaries. Always respond with valid JSON.".to_string(),
                        },
                        LiteLLMMessage {
                            role: "user".to_string(),
                            content: formatted_prompt,
                        },
                    ],
                    temperature: 0.3,
                    response_format: ResponseFormat {
                        format_type: "json_object".to_string(),
                    },
                };

                let response = self
                    .http_client
                    .post(format!("{}/chat/completions", self.litellm_base_url))
                    .json(&request)
                    .send()
                    .await
                    .context("Failed to send request to LiteLLM")?;

                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!(
                        "LiteLLM request failed with status {}: {}",
                        status,
                        error_text
                    ));
                }

                let litellm_response: LiteLLMResponse = response
                    .json()
                    .await
                    .context("Failed to parse LiteLLM response")?;

                let ai_content = litellm_response
                    .choices
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No choices in LiteLLM response"))?
                    .message
                    .content
                    .clone();

                self.parse_ai_response(ai_content)
            }
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn stage_files(&self, paths: Vec<String>) -> Result<()> {
        match &self.repo {
            Some(repo) => repo.stage_files(paths),
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn stage_all(&self) -> Result<()> {
        match &self.repo {
            Some(repo) => repo.stage_all(),
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn commit(&self, message: String) -> Result<CommitResult> {
        match &self.repo {
            Some(repo) => repo.commit(message),
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn push(&self) -> Result<()> {
        match &self.repo {
            Some(repo) => {
                let branch = repo.get_current_branch()?;
                repo.push("origin", &branch)
            }
            None => Err(anyhow::anyhow!("Git repository is not initialized")),
        }
    }

    pub async fn commit_and_push(&self, message: String) -> Result<CommitResult> {
        let result = self.commit(message).await?;
        self.push().await?;
        Ok(result)
    }

    fn load_prompt_template(&self) -> Result<String> {
        // Try to load from the prompts directory
        let prompt_path = PathBuf::from("prompts/git-summary.md");
        
        fs::read_to_string(&prompt_path)
            .with_context(|| format!("Failed to read prompt template from {:?}", prompt_path))

    }

    fn parse_ai_response(&self, response: String) -> Result<CommitSummary> {
        // Try to parse as JSON
        let summary: CommitSummary = serde_json::from_str(&response)
            .with_context(|| format!("Failed to parse AI response as JSON: {}", response))?;

        // Validate the response
        if summary.summary.is_empty() {
            return Err(anyhow::anyhow!("AI response missing summary"));
        }

        if summary.bullets.is_empty() {
            return Err(anyhow::anyhow!("AI response missing bullet points"));
        }

        if summary.suggested_message.is_empty() {
            return Err(anyhow::anyhow!("AI response missing suggested message"));
        }

        Ok(summary)
    }
}