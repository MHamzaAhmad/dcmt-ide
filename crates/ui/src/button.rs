use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

#[component]
pub fn Button(
    children: Element,
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = ButtonSize::Medium)] size: ButtonSize,
    #[props(default = false)] disabled: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let base_classes = "inline-flex items-center justify-center font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 rounded-md";
    
    let variant_classes = match variant {
        ButtonVariant::Primary => "bg-blue-600 text-white hover:bg-blue-700 focus-visible:ring-blue-500",
        ButtonVariant::Secondary => "bg-gray-200 text-gray-900 hover:bg-gray-300 focus-visible:ring-gray-500 dark:bg-gray-700 dark:text-gray-100 dark:hover:bg-gray-600",
        ButtonVariant::Ghost => "hover:bg-gray-100 hover:text-gray-900 dark:hover:bg-gray-800 dark:hover:text-gray-100",
        ButtonVariant::Danger => "bg-red-600 text-white hover:bg-red-700 focus-visible:ring-red-500",
    };
    
    let size_classes = match size {
        ButtonSize::Small => "h-8 px-3 text-sm",
        ButtonSize::Medium => "h-10 px-4 py-2",
        ButtonSize::Large => "h-12 px-6 text-lg",
    };
    
    let class_str = format!(
        "{} {} {} {}",
        base_classes,
        variant_classes,
        size_classes,
        class.unwrap_or_default()
    );

    rsx! {
        button {
            class: "{class_str}",
            disabled: disabled,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },
            {children}
        }
    }
}