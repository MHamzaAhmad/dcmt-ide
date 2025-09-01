use crate::*;

#[component]
pub fn ResizablePanel(
    children: Element,
    #[props(default = 300)] min_width: u32,
    #[props(default = 800)] max_width: u32,
    #[props(default)] class: Option<String>,
) -> Element {
    let mut width = use_signal(|| 400u32);
    let mut is_resizing = use_signal(|| false);
    
    let class_str = format!(
        "relative flex flex-col h-full {}",
        class.unwrap_or_default()
    );
    
    rsx! {
        div {
            class: "{class_str}",
            style: "width: {width}px",
            
            // Panel content
            div {
                class: "flex-1 overflow-auto",
                {children}
            }
            
            // Resize handle
            div {
                class: "absolute right-0 top-0 w-1 h-full cursor-col-resize bg-transparent hover:bg-blue-500 transition-colors",
                onmousedown: move |_| {
                    is_resizing.set(true);
                },
                onmousemove: move |evt| {
                    if *is_resizing.read() {
                        let new_width = evt.client_coordinates().x as u32;
                        width.set(new_width.clamp(min_width, max_width));
                    }
                },
                onmouseup: move |_| {
                    is_resizing.set(false);
                }
            }
        }
    }
}

#[component]
pub fn PanelGroup(
    children: Element,
    #[props(default = "horizontal".to_string())] direction: String,
) -> Element {
    let class_str = if direction == "horizontal" {
        "flex flex-row h-full w-full"
    } else {
        "flex flex-col h-full w-full"
    };
    
    rsx! {
        div {
            class: "{class_str}",
            {children}
        }
    }
}