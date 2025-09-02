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
                "absolute bottom-0 left-0 right-0 h-12 bg-white dark:bg-zinc-900 border-t border-zinc-200 dark:border-zinc-800 shadow-2xl z-50 transition-all duration-300"
            } else {
                "absolute bottom-0 left-0 right-0 h-80 bg-white dark:bg-zinc-900 border-t border-zinc-200 dark:border-zinc-800 shadow-2xl z-50 transition-all duration-300"
            },
            
            // Header with controls - modern minimal design
            div { class: "flex items-center justify-between px-4 py-2 border-b border-zinc-100 dark:border-zinc-800 bg-zinc-50/80 dark:bg-zinc-900/80 backdrop-blur-sm",
                div { class: "flex items-center space-x-3",
                    // Modern chat icon with gradient
                    div { class: "flex items-center justify-center w-8 h-8 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg shadow-sm",
                        svg {
                            class: "w-4 h-4 text-white",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                        }
                    }
                    div { class: "flex flex-col",
                        h3 { class: "text-sm font-semibold text-zinc-900 dark:text-zinc-100",
                            "AI Assistant"
                        }
                        span { class: "text-xs text-zinc-500 dark:text-zinc-400",
                            "Ready to help"
                        }
                    }
                }
                
                div { class: "flex items-center space-x-2",
                    // Minimize/Maximize button - modern rounded design
                    button {
                        class: "flex items-center justify-center w-8 h-8 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-full transition-all duration-200 hover:scale-105",
                        onclick: move |_| {
                            let current = *is_minimized.read();
                            is_minimized.set(!current);
                        },
                        title: if *is_minimized.read() { "Maximize" } else { "Minimize" },
                        
                        if *is_minimized.read() {
                            // Maximize icon - cleaner chevron up
                            svg {
                                class: "w-4 h-4 text-zinc-600 dark:text-zinc-300",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2.5",
                                view_box: "0 0 24 24",
                                polyline { points: "18 15 12 9 6 15" }
                            }
                        } else {
                            // Minimize icon - cleaner chevron down
                            svg {
                                class: "w-4 h-4 text-zinc-600 dark:text-zinc-300",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2.5",
                                view_box: "0 0 24 24",
                                polyline { points: "6 9 12 15 18 9" }
                            }
                        }
                    }
                    
                    // Close button - modern with hover state
                    button {
                        class: "flex items-center justify-center w-8 h-8 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-full transition-all duration-200 hover:scale-105 group",
                        onclick: move |_| {
                            show.set(false);
                        },
                        title: "Close Chat",
                        
                        // X icon with hover color change
                        svg {
                            class: "w-4 h-4 text-zinc-400 dark:text-zinc-500 group-hover:text-red-500 dark:group-hover:text-red-400 transition-colors",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.5",
                            view_box: "0 0 24 24",
                            line { x1: "18", y1: "6", x2: "6", y2: "18" }
                            line { x1: "6", y1: "6", x2: "18", y2: "18" }
                        }
                    }
                }
            }
            
            // Chat content (only shown when not minimized) - with proper padding and styling
            if !*is_minimized.read() {
                div { class: "flex-1 overflow-hidden bg-white dark:bg-zinc-900",
                    div { class: "h-full p-4",
                        WebAIChatInterface { capabilities: capabilities }
                    }
                }
            }
        }
    }
}