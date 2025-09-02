// This file has been deprecated and split into modular components.
// Please use the new modular approach:
// - transport.rs for WebSocket communication
// - file_operations.rs for file handling
// - components.rs for UI components
//
// For backward compatibility, these exports are maintained:

pub use super::components::{WebFileTree, WebFileTreeAdvanced, WebFileItem};
pub use super::transport::{TransportMessage, FileOp, FileInfo, FileTransportClient};
pub use super::file_operations::{
    upload_file_from_browser, download_file_content, delete_file_on_server,
    fetch_file_list, create_latex_file, create_bibliography_file,
    format_file_size, validate_file_name, is_latex_file, is_bibliography_file
};

// Legacy type aliases
pub type FileTransportClient = super::transport::FileTransportClient;

// Helper functions for backward compatibility
pub fn format_file_size(size: u64) -> String {
    super::file_operations::format_file_size(size)
}