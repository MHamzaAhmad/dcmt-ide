use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use wtransport::RecvStream;
use anyhow::Result;
use tracing::{debug, warn};
use latex_ide_yrs_collab::CollaborationEngine;

use crate::types::{WebTransportMessage, ServerEvent};
use crate::utils::read_message;

pub async fn handle_awareness_stream(
    stream: &mut RecvStream,
    _collab_engine: Arc<Mutex<CollaborationEngine>>,
    event_sender: broadcast::Sender<ServerEvent>,
) -> Result<()> {
    let message = read_message(stream).await?;
    
    match message {
        WebTransportMessage::Awareness { document_id, user_id, awareness_data } => {
            debug!("Updating awareness for user {} in document {}", user_id, document_id);
            
            let _ = event_sender.send(ServerEvent::AwarenessUpdate { 
                document_id, 
                user_id, 
                awareness: awareness_data 
            });
        }
        _ => {
            warn!("Unexpected message on awareness stream: {:?}", message);
        }
    }
    
    Ok(())
}