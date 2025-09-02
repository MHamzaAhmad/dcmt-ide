use crate::*;

#[derive(Props, Clone, PartialEq)]
pub struct GitStatusIndicatorProps {
    pub has_changes: bool,
    pub current_branch: String,
    pub session_branch: Option<String>,
}

#[component]
pub fn GitStatusIndicator(props: GitStatusIndicatorProps) -> Element {
    rsx! {
        div { class: "flex items-center space-x-2 text-xs",
            // Git status dot
            div { 
                class: format!(
                    "w-2 h-2 rounded-full {}",
                    if props.has_changes {
                        "bg-orange-500 animate-pulse"
                    } else {
                        "bg-green-500"
                    }
                )
            }
            
            // Branch info
            div { class: "text-gray-600 dark:text-gray-400",
                span { class: "font-mono", "{props.current_branch}" }
                if let Some(ref session) = props.session_branch {
                    span { class: "text-gray-500 dark:text-gray-500 ml-1",
                        "({session})"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct GitPanelToggleProps {
    pub visible: bool,
    pub has_changes: bool,
    pub on_toggle: EventHandler<()>,
}

#[component] 
pub fn GitPanelToggle(props: GitPanelToggleProps) -> Element {
    rsx! {
        button {
            class: format!(
                "relative p-2 rounded-md text-sm font-medium transition-colors {}",
                if props.visible {
                    "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300"
                } else {
                    "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 hover:text-gray-900 dark:hover:text-gray-100"
                }
            ),
            onclick: move |_| props.on_toggle.call(()),
            title: "Git Version Control",
            
            "🌿 Git"
            
            // Change indicator
            if props.has_changes {
                span { 
                    class: "absolute -top-1 -right-1 w-3 h-3 bg-orange-500 border-2 border-white dark:border-gray-900 rounded-full"
                }
            }
        }
    }
}