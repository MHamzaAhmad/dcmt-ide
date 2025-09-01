use crate::*;

#[derive(Clone, PartialEq)]
pub struct DropdownItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
}

#[component]
pub fn Dropdown(
    items: Vec<DropdownItem>,
    selected: Option<String>,
    onselect: EventHandler<String>,
    #[props(default = "Select...".to_string())] placeholder: String,
    #[props(default)] class: Option<String>,
) -> Element {
    let mut is_open = use_signal(|| false);
    
    let selected_item = items.iter()
        .find(|item| Some(&item.id) == selected.as_ref())
        .map(|item| item.label.clone())
        .unwrap_or(placeholder);
    
    let class_str = format!(
        "relative inline-block text-left {}",
        class.unwrap_or_default()
    );
    
    rsx! {
        div {
            class: "{class_str}",
            
            // Trigger button
            button {
                class: "inline-flex justify-between w-full rounded-md border border-gray-300 dark:border-gray-600 shadow-sm px-4 py-2 bg-white dark:bg-gray-800 text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                onclick: move |_| {
                    let current = *is_open.read();
                    is_open.set(!current);
                },
                
                span { {selected_item} }
                
                svg {
                    class: "-mr-1 ml-2 h-5 w-5",
                    xmlns: "http://www.w3.org/2000/svg",
                    view_box: "0 0 20 20",
                    fill: "currentColor",
                    path {
                        fill_rule: "evenodd",
                        d: "M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z",
                        clip_rule: "evenodd"
                    }
                }
            }
            
            // Dropdown menu
            if *is_open.read() {
                div {
                    class: "origin-top-right absolute right-0 mt-2 w-56 rounded-md shadow-lg bg-white dark:bg-gray-800 ring-1 ring-black ring-opacity-5 z-50",
                    div {
                        class: "py-1",
                        for item in items.iter() {
                            button {
                                class: "block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700",
                                onclick: {
                                    let id = item.id.clone();
                                    move |_| {
                                        onselect.call(id.clone());
                                        is_open.set(false);
                                    }
                                },
                                {item.label.clone()}
                            }
                        }
                    }
                }
            }
        }
    }
}