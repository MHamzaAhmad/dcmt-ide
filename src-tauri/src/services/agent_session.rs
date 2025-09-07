use std::time::{Duration, Instant};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::agent::{ChatSession, ChatMessage, AgentResult, AgentError, SessionInfo};

/// Manages chat sessions for multiple concurrent sessions with thread-safe access
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
            Ok(session_guard.messages.clone())
        } else {
            Err(AgentError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }
    
    /// Gets session information
    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        if let Some(session) = self.sessions.get(session_id) {
            let session_guard = session.read().await;
            Some(SessionInfo {
                id: session_guard.id.clone(),
                user_id: session_guard.user_id.clone(),
                message_count: session_guard.message_count(),
                created_at: format!("{:?}", session_guard.created_at),
                last_activity: format!("{:?}", session_guard.last_activity),
            })
        } else {
            None
        }
    }
    
    /// Lists all active sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut sessions = Vec::new();
        
        for entry in self.sessions.iter() {
            let session_guard = entry.value().read().await;
            sessions.push(SessionInfo {
                id: session_guard.id.clone(),
                user_id: session_guard.user_id.clone(),
                message_count: session_guard.message_count(),
                created_at: format!("{:?}", session_guard.created_at),
                last_activity: format!("{:?}", session_guard.last_activity),
            });
        }
        
        sessions
    }
    
    /// Removes a specific session
    pub async fn remove_session(&self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }
    
    /// Cleans up expired sessions based on max_session_age
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let now = Instant::now();
        let mut removed_count = 0;
        
        // Collect expired session IDs
        let mut expired_sessions = Vec::new();
        
        for entry in self.sessions.iter() {
            let session_guard = entry.value().read().await;
            if now.duration_since(session_guard.last_activity) > self.max_session_age {
                expired_sessions.push(entry.key().clone());
            }
        }
        
        // Remove expired sessions
        for session_id in expired_sessions {
            if self.sessions.remove(&session_id).is_some() {
                removed_count += 1;
                tracing::info!("Cleaned up expired session: {}", session_id);
            }
        }
        
        removed_count
    }
    
    
    /// Starts a background cleanup task that periodically removes expired sessions
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean up every 5 minutes
            
            loop {
                interval.tick().await;
                
                let removed_count = self.cleanup_expired_sessions().await;
                if removed_count > 0 {
                    tracing::info!("Session cleanup: removed {} expired sessions", removed_count);
                }
            }
        })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // Default 1 hour session age
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent::ChatMessage;

    #[tokio::test]
    async fn test_session_creation_and_retrieval() {
        let manager = SessionManager::new(Duration::from_secs(3600));
        
        // Create a new session
        let session_id = "test-session-1".to_string();
        let _session = manager.get_or_create_session(session_id.clone(), None).await;
        
        // Verify session exists
        assert_eq!(manager.sessions.len(), 1);
        
        // Get same session again
        let _same_session = manager.get_or_create_session(session_id.clone(), None).await;
        
        // Should be the same session (same count)
        assert_eq!(manager.sessions.len(), 1);
        
        // Add a message
        let message = ChatMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            ..Default::default()
        };
        
        manager.add_message(&session_id, message.clone()).await.unwrap();
        
        // Retrieve history
        let history = manager.get_history(&session_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, Some("Hello".to_string()));
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let manager = SessionManager::new(Duration::from_millis(100)); // Very short session age
        
        // Create a session
        let session_id = "test-session-cleanup".to_string();
        let _session = manager.get_or_create_session(session_id.clone(), None).await;
        
        assert_eq!(manager.sessions.len(), 1);
        
        // Wait for session to expire
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Clean up expired sessions
        let removed_count = manager.cleanup_expired_sessions().await;
        
        assert_eq!(removed_count, 1);
        assert_eq!(manager.sessions.len(), 0);
    }
}