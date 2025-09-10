pub mod agent;
pub mod files;
pub mod latex;
pub mod sse;
pub mod websocket;

pub use agent::agent_router;
pub use files::files_router;
pub use latex::latex_router;
pub use sse::sse_router;
pub use websocket::websocket_router;