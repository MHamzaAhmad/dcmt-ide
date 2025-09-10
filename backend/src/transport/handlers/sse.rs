use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        Response,
    },
};
use futures::stream::Stream;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::svc::AgentService;
use crate::model::agent::AgentEvent;

/// SSE endpoint for streaming agent events
/// This provides a unified streaming interface for both web and desktop platforms
pub async fn agent_events_handler(
    State(service): State<Arc<AgentService>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE connection established for agent events");

    // Create a subscription to all agent events
    let receiver = service.subscribe_to_all_events();
    
    // Convert the receiver to an SSE stream
    let stream = receiver_to_sse_stream(receiver);

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("ping"),
        )
}

/// SSE endpoint for streaming agent events for a specific session
pub async fn session_events_handler(
    axum::extract::Path(session_id): axum::extract::Path<String>,
    State(service): State<Arc<AgentService>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE connection established for session: {}", session_id);

    // Create a subscription to agent events for this session
    let receiver = service.subscribe_to_session_events(session_id.clone());
    
    // Convert the receiver to an SSE stream
    let stream = receiver_to_sse_stream(receiver);

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("ping"),
        )
}

/// Convert an mpsc receiver to an SSE stream
fn receiver_to_sse_stream(
    mut receiver: mpsc::UnboundedReceiver<AgentEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        while let Some(event) = receiver.recv().await {
            // Serialize the event to JSON
            match serde_json::to_string(&event) {
                Ok(json) => {
                    // Create SSE event with the event type as the event name
                    let sse_event = Event::default()
                        .event("agent-event")
                        .data(json);
                    
                    yield Ok(sse_event);
                }
                Err(e) => {
                    error!("Failed to serialize agent event: {}", e);
                    // Send error event
                    let error_event = Event::default()
                        .event("error")
                        .data(format!("Failed to serialize event: {}", e));
                    
                    yield Ok(error_event);
                }
            }
        }
        
        info!("SSE stream ended");
    }
}

/// Health check endpoint for SSE
pub async fn sse_health_handler() -> Response<String> {
    Response::new("SSE endpoint is healthy".to_string())
}