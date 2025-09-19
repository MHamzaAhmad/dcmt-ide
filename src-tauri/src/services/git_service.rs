use anyhow::{Context, Result};
// use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs;
use crate::repo::llm::LLMRepository;

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
    _workspace_path: PathBuf,
    litellm_base_url: String,
    cache_unstaged: Mutex<Option<(String, CommitSummary)>>,
    cache_staged: Mutex<Option<(String, CommitSummary)>>,
}

impl GitService {
    pub fn new(workspace_path: PathBuf, litellm_base_url: String) -> Result<Self> {
        let repo = if GitRepository::repository_exists(&workspace_path) {
            Some(Arc::new(GitRepository::new(workspace_path.clone())?))
        } else {
            None
        };
    // HTTP client not needed here; use LLMRepository internally

        Ok(Self {
            repo,
        _workspace_path: workspace_path,
            litellm_base_url,
            cache_unstaged: Mutex::new(None),
            cache_staged: Mutex::new(None),
        })
    }

    #[allow(dead_code)]
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

                // Compute stable signature
                let signature = compute_diff_signature(&diff_string);

                // Check cache
                let cached = if staged {
                    self.cache_staged.lock().unwrap().clone()
                } else {
                    self.cache_unstaged.lock().unwrap().clone()
                };
                if let Some((sig, summary)) = cached {
                    if sig == signature {
                        return Ok(summary);
                    }
                }

                if diff_string.is_empty() {
                    let no_changes = CommitSummary {
                        summary: "No changes to commit".to_string(),
                        bullets: vec!["No changes detected".to_string()],
                        suggested_message: "chore: no changes".to_string(),
                    };
                    if staged {
                        *self.cache_staged.lock().unwrap() = Some((signature, no_changes.clone()));
                    } else {
                        *self.cache_unstaged.lock().unwrap() = Some((signature, no_changes.clone()));
                    }
                    return Ok(no_changes);
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

                let llm_repo = LLMRepository::new(self.litellm_base_url.clone());
                let litellm_response: LiteLLMResponse = llm_repo
                    .create_chat_completion_with_body(&request)
                    .await
                    .context("Failed to request LiteLLM for commit summary")?;

                let ai_content = litellm_response
                    .choices
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No choices in LiteLLM response"))?
                    .message
                    .content
                    .clone();

                let parsed = self.parse_ai_response(ai_content)?;
                if staged {
                    *self.cache_staged.lock().unwrap() = Some((signature, parsed.clone()));
                } else {
                    *self.cache_unstaged.lock().unwrap() = Some((signature, parsed.clone()));
                }
                Ok(parsed)
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
        let prompt_path = PathBuf::from("../../prompts/git-summary.md");
        
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

fn compute_diff_signature(diff: &str) -> String {
    let hash = blake3::hash(diff.as_bytes());
    hash.to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_response() {
        let service = GitService {
            repo: None,
            _workspace_path: PathBuf::from("."),
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