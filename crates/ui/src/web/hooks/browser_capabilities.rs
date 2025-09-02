use dioxus_signals::{Signal};
use dioxus_hooks::{use_signal};

/// Browser capabilities detection
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BrowserCapabilities {
    pub webtransport_supported: bool,
    pub websocket_supported: bool,
    pub server_sent_events: bool,
    pub web_assembly: bool,
    pub local_storage: bool,
    pub indexed_db: bool,
}

/// Hook to detect browser capabilities (simplified for now)
#[allow(dead_code)]
pub fn use_browser_capabilities() -> Signal<BrowserCapabilities> {
    use_signal(|| BrowserCapabilities {
        webtransport_supported: true, // Assume supported for now
        websocket_supported: true,
        server_sent_events: true,
        web_assembly: true,
        local_storage: true,
        indexed_db: true,
    })
}