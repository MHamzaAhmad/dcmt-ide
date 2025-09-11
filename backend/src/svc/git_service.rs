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
    repo: Arc<GitRepository>,
    http_client: Client,
    litellm_base_url: String,
}

impl GitService {
    pub fn new(workspace_path: PathBuf, litellm_base_url: String) -> Result<Self> {
        let repo = Arc::new(GitRepository::new(workspace_path)?);
        let http_client = Client::new();

        Ok(Self {
            repo,
            http_client,
            litellm_base_url,
        })
    }

    pub async fn get_status(&self) -> Result<GitStatus> {
        self.repo.get_status()
    }

    pub async fn get_diff(&self, staged: bool) -> Result<GitDiff> {
        self.repo.get_diff(staged)
    }

    pub async fn generate_commit_summary(&self, staged: bool) -> Result<CommitSummary> {
        let diff_string = self.repo.get_diff_as_string(staged)?;
        
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

    pub async fn stage_files(&self, paths: Vec<String>) -> Result<()> {
        self.repo.stage_files(paths)
    }

    pub async fn stage_all(&self) -> Result<()> {
        self.repo.stage_all()
    }

    pub async fn commit(&self, message: String) -> Result<CommitResult> {
        self.repo.commit(message)
    }

    pub async fn push(&self) -> Result<()> {
        let branch = self.repo.get_current_branch()?;
        self.repo.push("origin", &branch)
    }

    pub async fn commit_and_push(&self, message: String) -> Result<CommitResult> {
        let result = self.commit(message).await?;
        self.push().await?;
        Ok(result)
    }

    fn load_prompt_template(&self) -> Result<String> {
        // Try to load from the prompts directory
        let prompt_path = PathBuf::from("prompts/git-summary.md");
        
        if prompt_path.exists() {
            fs::read_to_string(&prompt_path)
                .with_context(|| format!("Failed to read prompt template from {:?}", prompt_path))
        } else {
            // Fallback to embedded prompt
            Ok(self.get_default_prompt())
        }
    }

    fn get_default_prompt(&self) -> String {
        r#"# Git Change Summary Generator

You are analyzing git changes to help both developers and non-technical users understand what was modified.

## Your Task
Given the diff output below, provide:

1. **One-line summary** (plain English, accessible to everyone)
   - Describe WHAT changed and WHY it matters in simple terms
   - Max 100 characters
   - Example: "Updated the document editor to support real-time collaboration"

2. **Detailed changes** (3-7 bullet points)
   - Be specific but avoid technical jargon where possible
   - Focus on the purpose and impact of changes
   - Use present tense
   - Examples:
     - "• Added ability to export documents as PDF"
     - "• Fixed issue where images wouldn't load properly"
     - "• Improved performance of search functionality"

3. **Suggested commit message** (for version control)
   - Technical but concise (under 72 characters)
   - Follow conventional commit format if applicable
   - Example: "feat: add PDF export with custom formatting options"

## Input Diff
{diff_content}

## Expected Output Format (JSON)
{
  "summary": "A clear, non-technical summary of all changes",
  "bullets": [
    "• First change description",
    "• Second change description",
    "• Third change description"
  ],
  "suggestedMessage": "feat: technical commit message"
}"#.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_response() {
        let service = GitService {
            repo: Arc::new(GitRepository::new(PathBuf::from(".")).unwrap()),
            http_client: Client::new(),
            litellm_base_url: "http://localhost:4000".to_string(),
        };

        let valid_response = r#"{
            "summary": "Added version control integration",
            "bullets": ["• Added Git repository module", "• Created commit functionality"],
            "suggestedMessage": "feat: add git integration"
        }"#;

        let result = service.parse_ai_response(valid_response.to_string()).unwrap();
        assert_eq!(result.summary, "Added version control integration");
        assert_eq!(result.bullets.len(), 2);
        assert_eq!(result.suggested_message, "feat: add git integration");
    }
}