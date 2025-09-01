use dioxus::prelude::*;

#[component]
pub fn Input(
    #[props(default)] value: Option<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] input_type: Option<String>,
    #[props(default = false)] disabled: bool,
    onchange: EventHandler<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let class_str = format!(
        "flex h-10 w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:cursor-not-allowed disabled:opacity-50 {}",
        class.unwrap_or_default()
    );
    
    rsx! {
        input {
            class: "{class_str}",
            r#type: "{input_type.unwrap_or_else(|| \"text\".to_string())}",
            value: "{value.unwrap_or_default()}",
            placeholder: "{placeholder.unwrap_or_default()}",
            disabled: disabled,
            oninput: move |evt| {
                onchange.call(evt.value());
            }
        }
    }
}