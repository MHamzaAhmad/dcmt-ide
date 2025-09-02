use dioxus_signals::{Signal};
use dioxus_hooks::{use_signal};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
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

/// Hook to detect browser capabilities with proper WebTransport detection
#[allow(dead_code)]
pub fn use_browser_capabilities() -> Signal<BrowserCapabilities> {
    use_signal(|| {
        let mut capabilities = BrowserCapabilities {
            webtransport_supported: false,
            websocket_supported: true,
            server_sent_events: true,
            web_assembly: true,
            local_storage: true,
            indexed_db: true,
        };
        
        // Detect WebTransport support
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = window, js_name = WebTransport)]
                type WebTransportGlobal;
                
                #[wasm_bindgen(method, js_name = constructor)]
                fn constructor(this: &WebTransportGlobal) -> bool;
            }
            
            // Check if WebTransport is available
            let js_value = js_sys::Reflect::get(&web_sys::window().unwrap(), &"WebTransport".into());
            capabilities.webtransport_supported = js_value.is_ok() && !js_value.unwrap().is_undefined();
            
            tracing::info!("WebTransport supported: {}", capabilities.webtransport_supported);
        }
        
        capabilities
    })
}