// Module declarations
pub mod types;
pub mod server;
pub mod handlers;
pub mod utils;
pub mod git_manager;
pub mod websocket;

// Re-export public APIs
pub use types::*;
pub use server::WebTransportServer;
pub use git_manager::GitServerManager;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_type_conversion() {
        assert_eq!(StreamType::from_id(0), Some(StreamType::DocumentSync));
        assert_eq!(StreamType::from_id(1), Some(StreamType::Awareness));
        assert_eq!(StreamType::from_id(5), Some(StreamType::GitOps));
        assert_eq!(StreamType::from_id(6), None);
    }
}