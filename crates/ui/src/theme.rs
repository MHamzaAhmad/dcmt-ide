use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

impl Theme {
    pub fn toggle(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
    
    pub fn class(&self) -> &'static str {
        match self {
            Theme::Light => "",
            Theme::Dark => "dark",
        }
    }
}

#[component]
pub fn ThemeProvider(children: Element) -> Element {
    let theme = use_signal(|| {
        // Try to load theme from localStorage on web platform
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(stored_theme)) = storage.get_item("theme") {
                        if stored_theme == "dark" {
                            return Theme::Dark;
                        }
                    }
                }
            }
        }
        Theme::default()
    });
    
    // Apply theme class to document root and persist to localStorage
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let theme_value = *theme.read();
            
            // Apply class to document root
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(root) = document.document_element() {
                        let class_list = root.class_list();
                        if theme_value == Theme::Dark {
                            let _ = class_list.add_1("dark");
                        } else {
                            let _ = class_list.remove_1("dark");
                        }
                    }
                }
                
                // Persist to localStorage
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("theme", if theme_value == Theme::Dark { "dark" } else { "light" });
                }
            }
        }
    });
    
    use_context_provider(|| theme);
    
    // Still apply the class to this div for fallback
    rsx! {
        div {
            class: "{theme.read().class()}",
            {children}
        }
    }
}

pub fn use_theme() -> Signal<Theme> {
    use_context::<Signal<Theme>>()
}