use crate::*;

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
        ButtonVariant::Primary => "bg-zinc-900 text-zinc-50 hover:bg-zinc-900/90 dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-50/90 focus-visible:ring-zinc-900 dark:focus-visible:ring-zinc-300",
        ButtonVariant::Secondary => "bg-zinc-100 text-zinc-900 hover:bg-zinc-100/80 dark:bg-zinc-800 dark:text-zinc-50 dark:hover:bg-zinc-800/80 focus-visible:ring-zinc-900 dark:focus-visible:ring-zinc-300",
        ButtonVariant::Ghost => "hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-50",
        ButtonVariant::Danger => "bg-red-500 text-zinc-50 hover:bg-red-500/90 dark:bg-red-900 dark:text-zinc-50 dark:hover:bg-red-900/90 focus-visible:ring-red-500 dark:focus-visible:ring-red-900",
    };
    
    let size_classes = match size {
        ButtonSize::Small => "h-8 px-3 text-xs rounded-md",
        ButtonSize::Medium => "h-9 px-4 py-2 text-sm",
        ButtonSize::Large => "h-11 px-8 text-base",
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