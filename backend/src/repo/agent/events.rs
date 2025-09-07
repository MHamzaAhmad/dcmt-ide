use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use crate::model::agent::AgentEvent;

/// Broadcasts agent events to WebSocket subscribers
pub struct EventBroadcaster {
    /// Map of session_id -> sender channel for that session
    subscribers: Arc<DashMap<String, mpsc::UnboundedSender<AgentEvent>>>,
}

impl EventBroadcaster {
    /// Creates a new event broadcaster
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(DashMap::new()),
        }
    }
    
    /// Subscribes to events for a specific session
    /// Returns a receiver that will get events for this session
    pub fn subscribe(&self, session_id: String) -> mpsc::UnboundedReceiver<AgentEvent> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Remove any existing subscription for this session first
        self.unsubscribe(&session_id);
        
        // Add the new subscription
        self.subscribers.insert(session_id, sender);
        
        receiver
    }
    
    /// Unsubscribes from events for a specific session
    pub fn unsubscribe(&self, session_id: &str) {
        self.subscribers.remove(session_id);
    }
    
    /// Broadcasts an event to a specific session
    pub async fn broadcast(&self, session_id: &str, event: AgentEvent) {
        if let Some(entry) = self.subscribers.get(session_id) {
            let sender = entry.value();
            
            // Try to send the event
            if let Err(_) = sender.send(event) {
                // If sending fails, the receiver is likely dropped
                // Remove the stale subscription
                tracing::debug!("Removing stale subscription for session: {}", session_id);
                drop(entry); // Release the reference before removing
                self.subscribers.remove(session_id);
            }
        }
    }
    
    /// Broadcasts an event to all active sessions (useful for system-wide events)
    pub async fn broadcast_to_all(&self, event: AgentEvent) {
        let mut stale_sessions = Vec::new();
        
        for entry in self.subscribers.iter() {
            let session_id = entry.key().clone();
            let sender = entry.value();
            
            if let Err(_) = sender.send(event.clone()) {
                stale_sessions.push(session_id);
            }
        }
        
        // Clean up stale subscriptions
        for session_id in stale_sessions {
            self.subscribers.remove(&session_id);
        }
    }
    
    /// Gets the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
    
    /// Lists all active session IDs (useful for debugging)
    pub fn active_sessions(&self) -> Vec<String> {
        self.subscribers.iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
    
    /// Cleans up stale subscriptions by testing each one
    pub async fn cleanup_stale_subscriptions(&self) {
        let mut stale_sessions = Vec::new();
        
        for entry in self.subscribers.iter() {
            let session_id = entry.key().clone();
            let sender = entry.value();
            
            // Try to send a test event (this won't be received if channel is closed)
            if sender.is_closed() {
                stale_sessions.push(session_id);
            }
        }
        
        // Remove stale subscriptions
        for session_id in &stale_sessions {
            self.subscribers.remove(session_id);
        }
        
        if !stale_sessions.is_empty() {
            tracing::debug!("Cleaned up {} stale subscriptions", stale_sessions.len());
        }
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Event subscription handle that automatically unsubscribes when dropped
pub struct EventSubscription {
    session_id: String,
    receiver: Option<mpsc::UnboundedReceiver<AgentEvent>>,
    broadcaster: Arc<EventBroadcaster>,
}

impl EventSubscription {
    /// Creates a new subscription
    pub fn new(session_id: String, broadcaster: Arc<EventBroadcaster>) -> Self {
        let receiver = broadcaster.subscribe(session_id.clone());
        Self {
            session_id,
            receiver: Some(receiver),
            broadcaster,
        }
    }
    
    /// Takes the receiver (can only be called once)
    pub fn take_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<AgentEvent>> {
        self.receiver.take()
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        self.broadcaster.unsubscribe(&self.session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_broadcast_to_subscriber() {
        let broadcaster = EventBroadcaster::new();
        let session_id = "test-session".to_string();
        
        let mut receiver = broadcaster.subscribe(session_id.clone());
        
        let event = AgentEvent::JobQueued {
            job_id: "job-1".to_string(),
            session_id: session_id.clone(),
        };
        
        broadcaster.broadcast(&session_id, event.clone()).await;
        
        let received = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(received.is_ok());
        
        let received_event = received.unwrap().unwrap();
        match received_event {
            AgentEvent::JobQueued { job_id, session_id: recv_session_id } => {
                assert_eq!(job_id, "job-1");
                assert_eq!(recv_session_id, session_id);
            }
            _ => panic!("Unexpected event type"),
        }
    }
    
    #[tokio::test]
    async fn test_unsubscribe() {
        let broadcaster = EventBroadcaster::new();
        let session_id = "test-session".to_string();
        
        let _receiver = broadcaster.subscribe(session_id.clone());
        assert_eq!(broadcaster.subscriber_count(), 1);
        
        broadcaster.unsubscribe(&session_id);
        assert_eq!(broadcaster.subscriber_count(), 0);
    }
    
    #[tokio::test]
    async fn test_broadcast_to_nonexistent_session() {
        let broadcaster = EventBroadcaster::new();
        
        let event = AgentEvent::JobQueued {
            job_id: "job-1".to_string(),
            session_id: "nonexistent".to_string(),
        };
        
        // This should not panic
        broadcaster.broadcast("nonexistent", event).await;
    }
    
    #[tokio::test]
    async fn test_automatic_cleanup_on_drop() {
        let broadcaster = EventBroadcaster::new();
        let session_id = "test-session".to_string();
        
        {
            let _receiver = broadcaster.subscribe(session_id.clone());
            assert_eq!(broadcaster.subscriber_count(), 1);
        } // receiver is dropped here
        
        let event = AgentEvent::JobQueued {
            job_id: "job-1".to_string(),
            session_id: session_id.clone(),
        };
        
        // This should clean up the stale subscription
        broadcaster.broadcast(&session_id, event).await;
        assert_eq!(broadcaster.subscriber_count(), 0);
    }
}