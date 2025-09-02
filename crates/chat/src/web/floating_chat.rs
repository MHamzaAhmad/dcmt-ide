use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable, GlobalSignal};
use dioxus_hooks::use_signal;
use crate::web::{WebAIChatInterface, BrowserCapabilities};

#[component]
pub fn WebFloatingChatPanel(
    mut show: Signal<bool>,
    capabilities: Signal<BrowserCapabilities>,
) -> Element {
    let mut is_minimized = use_signal(|| false);
    
    rsx! {
        div { 
            class: if *is_minimized.read() {
                "absolute bottom-4 right-4 w-80 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-lg shadow-lg z-10 transition-all duration-300"
            } else {
                "absolute bottom-4 right-4 w-80 h-96 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-lg shadow-lg z-10 transition-all duration-300"
            },
            
            // Header with controls
            div { class: "flex items-center justify-between p-3 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-800 rounded-t-lg",
                div { class: "flex items-center space-x-2",
                    // Chat icon
                    svg {
                        class: "w-4 h-4 text-zinc-500 dark:text-zinc-400",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                    }
                    h3 { class: "text-sm font-medium text-zinc-900 dark:text-zinc-100",
                        "AI Chat"
                    }
                }
                
                div { class: "flex items-center space-x-1",
                    // Minimize/Maximize button
                    button {
                        class: "p-1 hover:bg-zinc-200 dark:hover:bg-zinc-700 rounded transition-colors",
                        onclick: move |_| {
                            let current = *is_minimized.read();
                            is_minimized.set(!current);
                        },
                        title: if *is_minimized.read() { "Maximize" } else { "Minimize" },
                        
                        if *is_minimized.read() {
                            // Maximize icon
                            svg {
                                class: "w-3 h-3 text-zinc-500 dark:text-zinc-400",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                view_box: "0 0 24 24",
                                polyline { points: "15 3 21 3 21 9" }
                                polyline { points: "9 21 3 21 3 15" }
                                line { x1: "21", y1: "3", x2: "14", y2: "10" }
                                line { x1: "3", y1: "21", x2: "10", y2: "14" }
                            }
                        } else {
                            // Minimize icon
                            svg {
                                class: "w-3 h-3 text-zinc-500 dark:text-zinc-400",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                view_box: "0 0 24 24",
                                line { x1: "5", y1: "12", x2: "19", y2: "12" }
                            }
                        }
                    }
                    
                    // Close button
                    button {
                        class: "p-1 hover:bg-zinc-200 dark:hover:bg-zinc-700 rounded transition-colors",
                        onclick: move |_| {
                            show.set(false);
                        },
                        title: "Close Chat",
                        
                        // X icon
                        svg {
                            class: "w-3 h-3 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            line { x1: "18", y1: "6", x2: "6", y2: "18" }
                            line { x1: "6", y1: "6", x2: "18", y2: "18" }
                        }
                    }
                }
            }
            
            // Chat content (only shown when not minimized)
            if !*is_minimized.read() {
                div { class: "flex-1 h-full",
                    WebAIChatInterface { capabilities: capabilities }
                }
            }
        }
    }
}