pub mod files;
pub mod latex;
pub mod websocket;

pub use files::files_router;
pub use latex::latex_router;
pub use websocket::websocket_router;