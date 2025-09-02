// DEPRECATED: This file has been split into modular components
// Please use the new modules instead:
// - transport.rs for WebSocket communication
// - file_operations.rs for file handling  
// - components.rs for UI components
//
// This file is kept for backward compatibility only.

// Re-export all functionality from the new modular components
pub use super::components::{WebFileTree, WebFileTreeAdvanced};
pub use super::transport::{TransportMessage, FileOp, FileInfo, FileTransportClient};
pub use super::file_operations::{
    upload_file_from_browser, download_file_content, delete_file_on_server,
    fetch_file_list, create_latex_file, create_bibliography_file,
    format_file_size, validate_file_name, is_latex_file, is_bibliography_file
};