//! Web-specific file manager components

// Core modules
pub mod transport;
pub mod file_operations;
pub mod components;

// Legacy modules (deprecated)
pub mod file_tree;
pub mod webtransport_fs;

// Re-export new modular components
pub use components::{WebFileTree, WebFileTreeAdvanced};
pub use transport::{TransportMessage, FileOp, FileInfo, FileTransportClient};
pub use file_operations::{
    upload_file_from_browser, download_file_content, delete_file_on_server,
    fetch_file_list, create_latex_file, create_bibliography_file,
    format_file_size, validate_file_name, is_latex_file, is_bibliography_file
};

// Legacy exports for backward compatibility
pub use webtransport_fs::WebTransportFileSystem;