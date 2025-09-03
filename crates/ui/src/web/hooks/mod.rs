//! Web-specific hooks

pub mod browser_capabilities;

pub use browser_capabilities::{BrowserCapabilities, use_browser_capabilities, detect_webtransport_support};