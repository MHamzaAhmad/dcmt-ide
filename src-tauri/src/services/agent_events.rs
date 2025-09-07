use std::sync::Arc;
use dashmap::DashMap;
use tauri::{AppHandle, Emitter};
use crate::models::agent::AgentEvent;

/// Broadcasts agent events to the frontend using Tauri's event system
pub struct EventBroadcaster {
    app_handle: AppHandle,
    /// Keep track of active subscriptions for cleanup
    active_sessions: Arc<DashMap<String, bool>>,
}

impl EventBroadcaster {
    /// Creates a new event broadcaster with Tauri app handle
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            active_sessions: Arc::new(DashMap::new()),
        }
    }
    
    /// Subscribes to events for a specific session
    /// This marks the session as active for event broadcasting
    pub fn subscribe(&self, session_id: String) {
        self.active_sessions.insert(session_id.clone(), true);
        tracing::debug!("Subscribed to agent events for session: {}", session_id);
    }
    
    /// Unsubscribes from events for a specific session
    pub fn unsubscribe(&self, session_id: &str) {
        self.active_sessions.remove(session_id);
        tracing::debug!("Unsubscribed from agent events for session: {}", session_id);
    }
    
    /// Broadcasts an event to a specific session using Tauri events
    pub async fn broadcast(&self, session_id: &str, event: AgentEvent) {
        // Check if session is still active
        if !self.active_sessions.contains_key(session_id) {
            tracing::debug!("Session {} not subscribed, skipping event broadcast", session_id);
            return;
        }
        
        let event_name = format!("agent-event-{}", session_id);
        
        // Emit the event to the frontend
        if let Err(e) = self.app_handle.emit(&event_name, &event) {
            tracing::error!("Failed to emit agent event for session {}: {}", session_id, e);
            
            // If emission fails, remove the subscription as the frontend might be disconnected
            self.unsubscribe(session_id);
        } else {
            tracing::debug!("Emitted event {:?} for session: {}", event, session_id);
        }
    }
    
}

impl Clone for EventBroadcaster {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            active_sessions: self.active_sessions.clone(),
        }
    }
}

