use dioxus_signals::{Signal};
use dioxus_hooks::{use_signal};

#[cfg(target_arch = "wasm32")]
use {
    js_sys, web_sys,
};

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

/// Standalone function to detect WebTransport support (for use in non-hook contexts)
pub fn detect_webtransport_support() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Reflect;
        use wasm_bindgen::JsValue;
        
        if let Some(window) = web_sys::window() {
            let js_value = Reflect::get(&window, &JsValue::from_str("WebTransport"));
            let supported = js_value.is_ok() && !js_value.unwrap().is_undefined();
            tracing::debug!("WebTransport support detected: {}", supported);
            supported
        } else {
            false
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Hook to detect browser capabilities with proper WebTransport detection
#[allow(dead_code)]
pub fn use_browser_capabilities() -> Signal<BrowserCapabilities> {
    use_signal(|| {
        let webtransport_supported = detect_webtransport_support();
        
        let capabilities = BrowserCapabilities {
            webtransport_supported,
            websocket_supported: true,
            server_sent_events: true,
            web_assembly: true,
            local_storage: true,
            indexed_db: true,
        };
        
        tracing::info!("Browser capabilities detected - WebTransport: {}, WebSocket: {}", 
                      capabilities.webtransport_supported, capabilities.websocket_supported);
        
        capabilities
    })
}