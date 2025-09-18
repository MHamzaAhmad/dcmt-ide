use anyhow::{Context, Result};
// use reqwest::Client; // not needed anymore
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;

use crate::repo::git_repository::{
    CommitResult, GitDiff, GitRepository, GitStatus,
};
use crate::repo::llm::{LLMRepository, ChatCompletionRequest};
use crate::model::agent::{ChatMessage as AgentChatMessage, ResponseFormat as AgentResponseFormat};

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
    _workspace_path: PathBuf,
    llm_repo: Arc<LLMRepository>,
}

impl GitService {
    pub fn new(workspace_path: PathBuf, litellm_base_url: String) -> Result<Self> {
        let repo = if GitRepository::repository_exists(&workspace_path) {
            Some(Arc::new(GitRepository::new(workspace_path.clone())?))
        } else {
            None
        };
    let llm_repo = Arc::new(LLMRepository::new(litellm_base_url.clone())?);

        Ok(Self {
            repo,
            _workspace_path: workspace_path,
            llm_repo,
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
        tracing::info!("GitService: generate_commit_summary called",);
        match &self.repo {
            Some(repo) => {
                tracing::info!("GitService: generating diff string",);
                let diff_string = repo.get_diff_as_string(staged)?;
                tracing::info!(
                    "GitService: diff ready (staged={}, bytes={})",
                    staged,
                    diff_string.len()
                );

                if diff_string.is_empty() {
                    return Ok(CommitSummary {
                        summary: "No changes to commit".to_string(),
                        bullets: vec!["No changes detected".to_string()],
                        suggested_message: "chore: no changes".to_string(),
                    });
                }

    tracing::info!("GitService: loading prompt template",);
    let prompt = self.load_prompt_template()?;
                let formatted_prompt = prompt.replace("{diff_content}", &diff_string);

                // Build typed non-streaming chat request
                let messages = vec![
                    AgentChatMessage { role: "system".into(), content: Some("You are a helpful assistant that generates git commit summaries. Always respond with valid JSON.".into()), tool_calls: None, tool_call_id: None },
                    AgentChatMessage { role: "user".into(), content: Some(formatted_prompt), tool_calls: None, tool_call_id: None },
                ];
                let req = ChatCompletionRequest {
                    model: "gpt-4o-mini".into(),
                    messages,
                    tools: None,
                    tool_choice: None,
                    stream: Some(false),
                    temperature: Some(0.3),
                    max_tokens: None,
                    response_format: Some(AgentResponseFormat { format_type: "json_object".into() }),
                };

                tracing::info!("GitService: calling LLM for summary (non-streaming)");
                let litellm_response = self.llm_repo
                    .create_chat_completion(&req)
                    .await
                    .context("Failed to send request to LiteLLM")?;

                let ai_content = litellm_response
                    .choices
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No choices in LiteLLM response"))?
                    .message
                    .content
                    .clone()
                    .unwrap_or_default();
                tracing::info!(
                    "GitService: LLM responded (content_len={})",
                    ai_content.len()
                );
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
        // Try common relative locations first
        let candidates = [
            PathBuf::from("prompts/git-summary.md"),                // when CWD is repo root
            PathBuf::from("../prompts/git-summary.md"),            // when CWD is backend/
            PathBuf::from("../../prompts/git-summary.md"),         // when CWD is backend/target/debug
        ];

        for path in candidates.iter() {
            if path.exists() {
                tracing::info!("GitService: using prompt at {:?}", path);
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read prompt template from {:?}", path))?;
                return Ok(content);
            } else {
                tracing::debug!("GitService: prompt not found at {:?}", path);
            }
        }

        // Fallback to embedding the prompt at compile-time (repo-root/prompts)
        static EMBEDDED_PROMPT: &str = include_str!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../prompts/git-summary.md")
        );
        tracing::warn!(
            "GitService: using embedded prompt fallback (could not find prompt file at runtime)"
        );
        Ok(EMBEDDED_PROMPT.to_string())
    }

    fn parse_ai_response(&self, response: String) -> Result<CommitSummary> {
    tracing::info!("GitService: parsing AI response",);
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

        tracing::info!(
            "GitService: parsed summary (summary_chars={}, bullets={}, msg_chars={})",
            summary.summary.len(),
            summary.bullets.len(),
            summary.suggested_message.len()
        );
        Ok(summary)
    }
}