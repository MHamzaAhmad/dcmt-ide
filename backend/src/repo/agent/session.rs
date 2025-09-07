use std::time::{Duration, Instant};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::model::agent::{ChatSession, ChatMessage, AgentResult, AgentError};

/// Manages chat sessions for multiple users with concurrent access support
pub struct SessionManager {
    /// Thread-safe map of session_id -> ChatSession
    sessions: Arc<DashMap<String, Arc<RwLock<ChatSession>>>>,
    /// Maximum age before sessions are cleaned up
    max_session_age: Duration,
}

impl SessionManager {
    /// Creates a new session manager
    pub fn new(max_session_age: Duration) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            max_session_age,
        }
    }
    
    /// Gets an existing session or creates a new one
    pub async fn get_or_create_session(&self, session_id: String, user_id: Option<String>) -> Arc<RwLock<ChatSession>> {
        // Try to get existing session first
        if let Some(session) = self.sessions.get(&session_id) {
            // Update last activity
            {
                let mut session_guard = session.write().await;
                session_guard.last_activity = Instant::now();
            }
            return session.clone();
        }
        
        // Create new session
        let session = Arc::new(RwLock::new(ChatSession::new(session_id.clone(), user_id)));
        self.sessions.insert(session_id, session.clone());
        session
    }
    
    /// Adds a message to a session
    pub async fn add_message(&self, session_id: &str, message: ChatMessage) -> AgentResult<()> {
        if let Some(session) = self.sessions.get(session_id) {
            let mut session_guard = session.write().await;
            session_guard.add_message(message);
            Ok(())
        } else {
            Err(AgentError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }
    
    /// Gets the message history for a session
    pub async fn get_history(&self, session_id: &str) -> AgentResult<Vec<ChatMessage>> {
        if let Some(session) = self.sessions.get(session_id) {
            let session_guard = session.read().await;
            Ok(session_guard.get_messages().to_vec())
        } else {
            // Return empty history for new sessions instead of error
            Ok(Vec::new())
        }
    }
    
    /// Gets session information without messages
    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        if let Some(session) = self.sessions.get(session_id) {
            let session_guard = session.read().await;
            Some(SessionInfo {
                id: session_guard.id.clone(),
                user_id: session_guard.user_id.clone(),
                message_count: session_guard.messages.len(),
                created_at: session_guard.created_at,
                last_activity: session_guard.last_activity,
            })
        } else {
            None
        }
    }
    
    /// Lists all active sessions (useful for debugging/monitoring)
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut sessions = Vec::new();
        
        for entry in self.sessions.iter() {
            let session_guard = entry.read().await;
            sessions.push(SessionInfo {
                id: session_guard.id.clone(),
                user_id: session_guard.user_id.clone(),
                message_count: session_guard.messages.len(),
                created_at: session_guard.created_at,
                last_activity: session_guard.last_activity,
            });
        }
        
        sessions
    }
    
    /// Removes a specific session
    pub async fn remove_session(&self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }
    
    /// Cleans up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let _now = Instant::now();
        let mut removed_count = 0;
        
        // Collect expired session IDs first to avoid holding locks
        let mut expired_sessions = Vec::new();
        
        for entry in self.sessions.iter() {
            let session_guard = entry.read().await;
            if session_guard.is_expired(self.max_session_age) {
                expired_sessions.push(entry.key().clone());
            }
        }
        
        // Remove expired sessions
        for session_id in expired_sessions {
            if self.sessions.remove(&session_id).is_some() {
                removed_count += 1;
            }
        }
        
        removed_count
    }
    
    /// Gets the total number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    
    /// Starts a background task to periodically clean up expired sessions
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                let removed = self.cleanup_expired_sessions().await;
                if removed > 0 {
                    tracing::info!("Cleaned up {} expired sessions", removed);
                }
            }
        })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default
    }
}

/// Information about a session without the full message history
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub user_id: Option<String>,
    pub message_count: usize,
    #[serde(skip)]
    pub created_at: Instant,
    #[serde(skip)]
    pub last_activity: Instant,
}

impl SessionInfo {
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new(Duration::from_secs(60));
        let session_id = "test-session".to_string();
        
        let _session = manager.get_or_create_session(session_id.clone(), None).await;
        assert_eq!(manager.session_count(), 1);
        
        // Getting same session should return the same instance
        let _session2 = manager.get_or_create_session(session_id.clone(), None).await;
        assert_eq!(manager.session_count(), 1);
    }
    
    #[tokio::test]
    async fn test_message_handling() {
        let manager = SessionManager::new(Duration::from_secs(60));
        let session_id = "test-session".to_string();
        
        let message = ChatMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            ..Default::default()
        };
        
        manager.add_message(&session_id, message.clone()).await.unwrap();
        
        let history = manager.get_history(&session_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, message.content);
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let manager = SessionManager::new(Duration::from_millis(100));
        let session_id = "test-session".to_string();
        
        manager.get_or_create_session(session_id.clone(), None).await;
        assert_eq!(manager.session_count(), 1);
        
        // Wait for session to expire
        sleep(Duration::from_millis(150)).await;
        
        let removed = manager.cleanup_expired_sessions().await;
        assert_eq!(removed, 1);
        assert_eq!(manager.session_count(), 0);
    }
}