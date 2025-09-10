use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::model::agent::{ChatRequest, ChatResponse, AgentResult};
use crate::repo::agent::AgentRepo;

/// Service layer for agent operations with job queue management
pub struct AgentService {
    agent_repo: Arc<AgentRepo>,
    job_queue: Arc<Mutex<VecDeque<QueuedJob>>>,
}

#[derive(Debug, Clone)]
struct QueuedJob {
    job_id: String,
    request: ChatRequest,
}

impl AgentService {
    /// Creates a new agent service
    pub fn new(agent_repo: Arc<AgentRepo>) -> Self {
        Self {
            agent_repo,
            job_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    /// Queues a chat request for processing
    /// Returns immediately with a job ID while processing happens in the background
    pub async fn queue_chat(&self, request: ChatRequest) -> AgentResult<ChatResponse> {
        let job_id = Uuid::new_v4().to_string();
        
        let queued_job = QueuedJob {
            job_id: job_id.clone(),
            request: request.clone(),
        };
        
        // Add to queue
        {
            let mut queue = self.job_queue.lock().await;
            queue.push_back(queued_job);
        }
        
        // Clone values for response and async move
        let session_id_response = request.session_id.clone();
        let session_id_clone = request.session_id.clone();
        let job_id_clone = job_id.clone();
        
        // Process in background
        let agent_repo = self.agent_repo.clone();
        let job_queue = self.job_queue.clone();
        
        tokio::spawn(async move {
            // Process the job
            match agent_repo.process_chat(request).await {
                Ok(response) => {
                    tracing::info!("Job {} completed successfully for session {} with response: {}", 
                                 job_id_clone, session_id_clone, response);
                    
                    // The AgentRepo already emits JobComplete event internally,
                    // but we log here for verification
                }
                Err(e) => {
                    tracing::error!("Job {} failed for session {}: {}", 
                                  job_id_clone, session_id_clone, e);
                    
                    // Notify about the error via event broadcaster
                    agent_repo.get_event_broadcaster()
                        .broadcast(&session_id_clone, crate::model::agent::AgentEvent::Error {
                            message: format!("Job failed: {}", e),
                            metadata: crate::model::agent::EventMetadata::new(job_id_clone.clone()),
                        })
                        .await;
                }
            }
            
            // Remove completed job from queue (optional cleanup)
            Self::remove_completed_job(&job_queue, &job_id_clone).await;
        });
        
        Ok(ChatResponse {
            session_id: session_id_response,
            message: "Request queued for processing. You'll receive updates via WebSocket.".to_string(),
            job_id,
        })
    }
    
    /// Removes a completed job from the queue
    async fn remove_completed_job(job_queue: &Arc<Mutex<VecDeque<QueuedJob>>>, job_id: &str) {
        let mut queue = job_queue.lock().await;
        queue.retain(|job| job.job_id != job_id);
    }
    
    /// Gets the current queue size (useful for monitoring)
    pub async fn queue_size(&self) -> usize {
        let queue = self.job_queue.lock().await;
        queue.len()
    }
    
    /// Lists pending jobs (useful for debugging/monitoring)
    pub async fn list_pending_jobs(&self) -> Vec<PendingJobInfo> {
        let queue = self.job_queue.lock().await;
        queue.iter()
            .map(|job| PendingJobInfo {
                job_id: job.job_id.clone(),
                session_id: job.request.session_id.clone(),
                model: job.request.model.clone(),
                message_preview: job.request.message.chars().take(100).collect(),
            })
            .collect()
    }
    
    /// Gets agent repository for direct access (if needed)
    pub fn get_agent_repo(&self) -> Arc<AgentRepo> {
        self.agent_repo.clone()
    }
    
    /// Processes a chat request synchronously (useful for testing)
    /// Note: This bypasses the job queue and processes immediately
    pub async fn process_chat_sync(&self, request: ChatRequest) -> AgentResult<String> {
        self.agent_repo.process_chat(request).await
    }
    
    /// Gets session information
    pub async fn get_session_info(&self, session_id: &str) -> Option<crate::repo::agent::SessionInfo> {
        self.agent_repo.get_session_manager()
            .get_session_info(session_id)
            .await
    }
    
    /// Lists all active sessions
    pub async fn list_active_sessions(&self) -> Vec<crate::repo::agent::SessionInfo> {
        self.agent_repo.get_session_manager()
            .list_sessions()
            .await
    }
    
    /// Subscribes to events for a session (returns event receiver)
    pub fn subscribe_to_session_events(&self, session_id: String) -> tokio::sync::mpsc::UnboundedReceiver<crate::model::agent::AgentEvent> {
        self.agent_repo.get_event_broadcaster()
            .subscribe(session_id)
    }
    
    /// Subscribes to all agent events (global subscription)
    pub fn subscribe_to_all_events(&self) -> tokio::sync::mpsc::UnboundedReceiver<crate::model::agent::AgentEvent> {
        // Create a unique ID for this global subscription
        let global_id = format!("global-{}", uuid::Uuid::new_v4());
        self.agent_repo.get_event_broadcaster()
            .subscribe(global_id)
    }
    
    /// Gets available tool definitions
    pub fn get_available_tools(&self) -> Vec<crate::model::agent::ToolDefinition> {
        self.agent_repo.get_tool_definitions()
    }
    
    /// Cleans up expired sessions manually
    pub async fn cleanup_expired_sessions(&self) -> usize {
        self.agent_repo.get_session_manager()
            .cleanup_expired_sessions()
            .await
    }
}

/// Information about a pending job in the queue
#[derive(Debug, Clone)]
pub struct PendingJobInfo {
    pub job_id: String,
    pub session_id: String,
    pub model: String,
    pub message_preview: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_size() {
        // Simple test that doesn't require tempfile
        let temp_dir = std::env::temp_dir().join("test_agent");
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        let agent_repo = Arc::new(
            AgentRepo::new(temp_dir, "http://localhost:4000".to_string())
                .await
                .unwrap()
        );
        
        let service = AgentService::new(agent_repo);
        
        assert_eq!(service.queue_size().await, 0);
    }
}