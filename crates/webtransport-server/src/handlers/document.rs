use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use wtransport::RecvStream;
use anyhow::Result;
use tracing::{debug, warn};
use latex_ide_yrs_collab::CollaborationEngine;

use crate::types::{WebTransportMessage, ServerEvent};
use crate::utils::read_message;

pub async fn handle_document_sync_stream(
    stream: &mut RecvStream,
    collab_engine: Arc<Mutex<CollaborationEngine>>,
    event_sender: broadcast::Sender<ServerEvent>,
) -> Result<()> {
    let message = read_message(stream).await?;
    
    match message {
        WebTransportMessage::DocumentSync { document_id, update, .. } => {
            debug!("Applying document update for {}", document_id);
            
            let engine = collab_engine.lock().await;
            engine.apply_update(&document_id, &update)?;
            
            let _ = event_sender.send(ServerEvent::DocumentUpdate { document_id, update });
        }
        _ => {
            warn!("Unexpected message on document sync stream: {:?}", message);
        }
    }
    
    Ok(())
}