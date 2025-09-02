use wtransport::RecvStream;
use anyhow::Result;
use tracing::{debug, warn};

use crate::types::WebTransportMessage;
use crate::utils::read_message;

pub async fn handle_ai_chat_stream(stream: &mut RecvStream) -> Result<()> {
    let message = read_message(stream).await?;
    
    match message {
        WebTransportMessage::AiChat { conversation_id, message, model_id: _ } => {
            debug!("AI chat message for conversation {}: {}", conversation_id, message);
            // Forward to AI handler
            // TODO: Implement AI chat handling
        }
        _ => {
            warn!("Unexpected message on AI chat stream: {:?}", message);
        }
    }
    
    Ok(())
}