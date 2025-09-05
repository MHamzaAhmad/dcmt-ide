use crate::*;

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
        "flex h-9 w-full rounded-md border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 px-3 py-2 text-sm text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-500 dark:placeholder:text-zinc-400 focus:outline-none focus:ring-2 focus:ring-zinc-900 dark:focus:ring-zinc-300 focus:border-transparent disabled:cursor-not-allowed disabled:opacity-50 transition-colors {}",
        class.unwrap_or_default()
    );
    
    let input_type_str = input_type.as_deref().unwrap_or("text");
    let value_str = value.as_deref().unwrap_or("");
    let placeholder_str = placeholder.as_deref().unwrap_or("");
    
    rsx! {
        input {
            class: "{class_str}",
            r#type: "{input_type_str}",
            value: "{value_str}",
            placeholder: "{placeholder_str}",
            disabled: disabled,
            oninput: move |evt| {
                onchange.call(evt.value());
            }
        }
    }
}