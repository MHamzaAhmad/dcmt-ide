use crate::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[derive(Clone, PartialEq)]
pub struct DropdownItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub disabled: Option<bool>,
}

impl DropdownItem {
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            icon: None,
            description: None,
            disabled: None,
        }
    }
    
    pub fn with_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
    
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
}

#[component]
pub fn Dropdown(
    items: Vec<DropdownItem>,
    selected: Option<String>,
    onselect: EventHandler<String>,
    #[props(default = "Select...".to_string())] placeholder: String,
    #[props(default)] class: Option<String>,
    #[props(default)] size: Option<DropdownSize>,
    #[props(default)] variant: Option<DropdownVariant>,
) -> Element {
    let mut is_open = use_signal(|| false);
    let size = size.unwrap_or(DropdownSize::Medium);
    let variant = variant.unwrap_or(DropdownVariant::Default);
    
    // Close dropdown when clicking outside - using a simpler approach to avoid closure issues
    // We'll rely on event propagation stopping within the dropdown component instead
    
    let selected_item = items.iter()
        .find(|item| Some(&item.id) == selected.as_ref())
        .cloned();
    
    let display_text = selected_item.as_ref()
        .map(|item| item.label.clone())
        .unwrap_or(placeholder);
    
    let display_icon = selected_item.as_ref()
        .and_then(|item| item.icon.clone());
    
    let base_classes = "relative inline-block text-left";
    let button_classes = get_dropdown_button_classes(&size, &variant);
    let menu_classes = get_dropdown_menu_classes(&size);
    
    let class_str = format!("{} {}", base_classes, class.unwrap_or_default());
    
    rsx! {
        div {
            class: "{class_str}",
            tabindex: "0", // Make focusable
            onblur: move |_| {
                // Close dropdown when focus is lost
                is_open.set(false);
            },
            onclick: move |evt| {
                evt.stop_propagation();
            },
            
            // Trigger button
            button {
                class: "{button_classes}",
                onclick: move |_| {
                    let current = *is_open.read();
                    is_open.set(!current);
                },
                
                div { class: "flex items-center justify-between w-full",
                    div { class: "flex items-center space-x-2 min-w-0",
                        if let Some(icon) = &display_icon {
                            span { class: "text-sm", "{icon}" }
                        }
                        span { class: "truncate", "{display_text}" }
                    }
                    
                    // Chevron icon
                    svg {
                        class: format!("transition-transform duration-200 {}", 
                            if *is_open.read() { "rotate-180" } else { "rotate-0" }
                        ),
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        path { d: "m6 9 6 6 6-6" }
                    }
                }
            }
            
            // Dropdown menu
            if *is_open.read() {
                div {
                    class: "{menu_classes}",
                    div { class: "py-1",
                        for item in items.iter() {
                            DropdownMenuItem {
                                item: item.clone(),
                                size: size.clone(),
                                onclick: {
                                    let id = item.id.clone();
                                    let disabled = item.disabled.unwrap_or(false);
                                    move |_| {
                                        if !disabled {
                                            onselect.call(id.clone());
                                            is_open.set(false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum DropdownSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone, PartialEq)]
pub enum DropdownVariant {
    Default,
    Ghost,
    Outline,
}

#[component]
fn DropdownMenuItem(
    item: DropdownItem,
    size: DropdownSize,
    onclick: EventHandler<MouseEvent>
) -> Element {
    let disabled = item.disabled.unwrap_or(false);
    let item_classes = get_dropdown_item_classes(&size, disabled);
    
    rsx! {
        button {
            class: "{item_classes}",
            disabled: disabled,
            onclick: onclick,
            
            div { class: "flex items-center justify-between w-full",
                div { class: "flex items-center space-x-2 min-w-0",
                    if let Some(icon) = &item.icon {
                        span { class: "text-sm flex-shrink-0", "{icon}" }
                    }
                    div { class: "min-w-0",
                        div { class: "truncate font-medium", "{item.label}" }
                        if let Some(description) = &item.description {
                            div { class: "text-xs text-zinc-500 dark:text-zinc-400 truncate", 
                                "{description}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_dropdown_button_classes(size: &DropdownSize, variant: &DropdownVariant) -> String {
    let base = "inline-flex justify-between items-center w-full rounded-md text-sm font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-blue-400";
    
    let size_classes = match size {
        DropdownSize::Small => "px-2 py-1",
        DropdownSize::Medium => "px-3 py-2",
        DropdownSize::Large => "px-4 py-3",
    };
    
    let variant_classes = match variant {
        DropdownVariant::Default => "border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 hover:bg-zinc-50 dark:hover:bg-zinc-750",
        DropdownVariant::Ghost => "bg-transparent text-zinc-700 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800",
        DropdownVariant::Outline => "border border-zinc-300 dark:border-zinc-600 bg-transparent text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800",
    };
    
    format!("{} {} {}", base, size_classes, variant_classes)
}

fn get_dropdown_menu_classes(size: &DropdownSize) -> String {
    let base = "absolute right-0 mt-1 rounded-md shadow-lg bg-white dark:bg-zinc-800 ring-1 ring-black ring-opacity-5 dark:ring-zinc-700 z-50 border border-zinc-200 dark:border-zinc-700";
    
    let width_class = match size {
        DropdownSize::Small => "w-48",
        DropdownSize::Medium => "w-56", 
        DropdownSize::Large => "w-64",
    };
    
    format!("{} {}", base, width_class)
}

fn get_dropdown_item_classes(size: &DropdownSize, disabled: bool) -> String {
    let base = "block w-full text-left transition-colors";
    
    let size_classes = match size {
        DropdownSize::Small => "px-2 py-1.5 text-xs",
        DropdownSize::Medium => "px-3 py-2 text-sm",
        DropdownSize::Large => "px-4 py-3 text-base",
    };
    
    let state_classes = if disabled {
        "text-zinc-400 dark:text-zinc-500 cursor-not-allowed"
    } else {
        "text-zinc-900 dark:text-zinc-100 hover:bg-zinc-100 dark:hover:bg-zinc-700"
    };
    
    format!("{} {} {}", base, size_classes, state_classes)
}