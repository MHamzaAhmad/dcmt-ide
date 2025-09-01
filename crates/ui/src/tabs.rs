use dioxus::prelude::*;

#[derive(Clone)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub content: Element,
}

#[component]
pub fn Tabs(
    tabs: Vec<Tab>,
    #[props(default)] active_tab: Option<String>,
    onchange: EventHandler<String>,
) -> Element {
    let active = active_tab.unwrap_or_else(|| {
        tabs.first().map(|t| t.id.clone()).unwrap_or_default()
    });
    
    let active_content = tabs.iter()
        .find(|t| t.id == active)
        .map(|t| t.content.clone());
    
    rsx! {
        div {
            class: "w-full",
            
            // Tab list
            div {
                class: "border-b border-gray-200 dark:border-gray-700",
                nav {
                    class: "-mb-px flex space-x-8",
                    for tab in tabs.iter() {
                        button {
                            class: if tab.id == active {
                                "border-b-2 border-blue-500 py-2 px-1 text-sm font-medium text-blue-600 dark:text-blue-400"
                            } else {
                                "border-b-2 border-transparent py-2 px-1 text-sm font-medium text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                            },
                            onclick: {
                                let id = tab.id.clone();
                                move |_| {
                                    onchange.call(id.clone());
                                }
                            },
                            {tab.label.clone()}
                        }
                    }
                }
            }
            
            // Tab content
            div {
                class: "mt-4",
                {active_content}
            }
        }
    }
}