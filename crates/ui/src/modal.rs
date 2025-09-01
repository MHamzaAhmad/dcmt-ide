use crate::*;

#[component]
pub fn Modal(
    children: Element,
    is_open: bool,
    onclose: EventHandler<()>,
    #[props(default)] title: Option<String>,
) -> Element {
    if !is_open {
        return VNode::empty();
    }
    
    rsx! {
        div {
            class: "fixed inset-0 z-50 overflow-y-auto",
            
            // Backdrop
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 transition-opacity",
                onclick: move |_| {
                    onclose.call(());
                }
            }
            
            // Modal content
            div {
                class: "flex min-h-full items-center justify-center p-4",
                div {
                    class: "relative transform overflow-hidden rounded-lg bg-white dark:bg-gray-800 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg",
                    onclick: move |evt| {
                        evt.stop_propagation();
                    },
                    
                    // Header
                    if let Some(title) = title {
                        div {
                            class: "bg-white dark:bg-gray-800 px-4 pb-4 pt-5 sm:p-6 sm:pb-4",
                            h3 {
                                class: "text-lg font-semibold leading-6 text-gray-900 dark:text-gray-100",
                                {title}
                            }
                        }
                    }
                    
                    // Content
                    div {
                        class: "bg-white dark:bg-gray-800 px-4 pb-4 pt-5 sm:p-6",
                        {children}
                    }
                    
                    // Close button
                    button {
                        class: "absolute right-0 top-0 mr-4 mt-4 text-gray-400 hover:text-gray-500",
                        onclick: move |_| {
                            onclose.call(());
                        },
                        svg {
                            class: "h-6 w-6",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M6 18L18 6M6 6l12 12"
                            }
                        }
                    }
                }
            }
        }
    }
}