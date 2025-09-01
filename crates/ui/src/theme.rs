use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    let theme = use_signal(|| Theme::default());
    
    use_context_provider(|| theme);
    
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