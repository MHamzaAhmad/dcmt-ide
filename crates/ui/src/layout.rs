use dioxus::prelude::*;

#[component]
pub fn SplitView(
    left: Element,
    right: Element,
    #[props(default = 50.0)] initial_split: f32,
    #[props(default = true)] resizable: bool,
) -> Element {
    let mut split_position = use_signal(|| initial_split);
    let mut is_dragging = use_signal(|| false);
    
    rsx! {
        div {
            class: "flex h-full w-full relative",
            onmousemove: move |evt| {
                if *is_dragging.read() && resizable {
                    let x = evt.client_coordinates().x;
                    let width = evt.target().map(|t| t.client_width()).unwrap_or(1000) as f64;
                    let new_split = ((x / width) * 100.0) as f32;
                    split_position.set(new_split.clamp(20.0, 80.0));
                }
            },
            onmouseup: move |_| {
                is_dragging.set(false);
            },
            
            // Left panel
            div {
                class: "overflow-auto",
                style: "width: {split_position}%",
                {left}
            }
            
            // Divider
            if resizable {
                div {
                    class: "w-1 bg-gray-300 hover:bg-blue-500 cursor-col-resize transition-colors dark:bg-gray-600",
                    onmousedown: move |_| {
                        is_dragging.set(true);
                    }
                }
            }
            
            // Right panel
            div {
                class: "flex-1 overflow-auto",
                {right}
            }
        }
    }
}

#[component]
pub fn Sidebar(
    children: Element,
    #[props(default = true)] collapsed: bool,
    #[props(default = 250)] width: u32,
) -> Element {
    let width_style = if collapsed {
        "width: 60px".to_string()
    } else {
        format!("width: {}px", width)
    };
    
    rsx! {
        div {
            class: "h-full bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 transition-all duration-300",
            style: "{width_style}",
            {children}
        }
    }
}