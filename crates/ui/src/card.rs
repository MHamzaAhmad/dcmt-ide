use crate::*;

#[component]
pub fn Card(
    children: Element,
    #[props(default)] title: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let class_str = format!(
        "rounded-lg border bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 shadow-sm {}",
        class.unwrap_or_default()
    );
    
    rsx! {
        div {
            class: "{class_str}",
            if let Some(title) = title {
                div {
                    class: "px-6 py-4 border-b border-gray-200 dark:border-gray-700",
                    h3 {
                        class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                        {title}
                    }
                }
            }
            div {
                class: "p-6",
                {children}
            }
        }
    }
}