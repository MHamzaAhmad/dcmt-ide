use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable, GlobalSignal};
use crate::{Theme, use_theme};

#[derive(Clone, Debug)]
pub struct GitStatus {
    pub current_branch: String,
    pub session_branch: Option<String>,
    pub has_changes: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SidebarView {
    Explorer,
    Git,
    None,
}

#[component]
pub fn AppHeader(
    mut sidebar_view: Signal<SidebarView>,
    git_status: Signal<Option<GitStatus>>,
) -> Element {
    let mut theme = use_theme();
    
    rsx! {
        div { 
            class: "h-14 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex items-center justify-between px-4",
            
            // Left side - Panel toggles
            div { class: "flex items-center space-x-2",
                // File Explorer toggle
                button {
                    class: if *sidebar_view.read() == SidebarView::Explorer {
                        "p-2.5 rounded-md bg-zinc-900 text-zinc-50 dark:bg-zinc-50 dark:text-zinc-900 transition-colors"
                    } else {
                        "p-2.5 rounded-md text-zinc-600 hover:text-zinc-900 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:text-zinc-100 dark:hover:bg-zinc-800 transition-colors"
                    },
                    onclick: move |_| {
                        let current = *sidebar_view.read();
                        if current == SidebarView::Explorer {
                            sidebar_view.set(SidebarView::None);
                        } else {
                            sidebar_view.set(SidebarView::Explorer);
                        }
                    },
                    title: "File Explorer",
                    
                    // Folder icon
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path {
                            d: "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                        }
                    }
                }
                
                // Git panel toggle
                button {
                    class: if *sidebar_view.read() == SidebarView::Git {
                        "p-2.5 rounded-md bg-zinc-900 text-zinc-50 dark:bg-zinc-50 dark:text-zinc-900 transition-colors relative"
                    } else {
                        "p-2.5 rounded-md text-zinc-600 hover:text-zinc-900 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:text-zinc-100 dark:hover:bg-zinc-800 transition-colors relative"
                    },
                    onclick: move |_| {
                        let current = *sidebar_view.read();
                        if current == SidebarView::Git {
                            sidebar_view.set(SidebarView::None);
                        } else {
                            sidebar_view.set(SidebarView::Git);
                        }
                    },
                    title: "Git",
                    
                    // Git branch icon
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path {
                            d: "M6 3v12"
                        }
                        circle { cx: "18", cy: "6", r: "3" }
                        circle { cx: "6", cy: "15", r: "3" }
                        path {
                            d: "M18 9a9 9 0 01-9 9"
                        }
                    }
                    
                    // Show indicator if there are changes
                    if let Some(ref status) = *git_status.read() {
                        if status.has_changes {
                            div {
                                class: "absolute -top-1 -right-1 w-2 h-2 bg-orange-500 rounded-full",
                            }
                        }
                    }
                }
                
                // Separator
                div { class: "w-px h-6 bg-zinc-300 dark:bg-zinc-700 mx-4" }
                
                // Project name / title
                h1 { class: "text-sm font-medium text-zinc-700 dark:text-zinc-300",
                    "LaTeX Editor"
                }
            }
            
            // Right side - Theme toggle
            div { class: "flex items-center",
                // Theme toggle button
                button {
                    class: "p-2.5 rounded-md hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors",
                    onclick: move |_| {
                        let new_theme = theme.read().toggle();
                        theme.set(new_theme);
                    },
                    title: if *theme.read() == Theme::Light { "Switch to Dark Mode" } else { "Switch to Light Mode" },
                    
                    if *theme.read() == Theme::Light {
                        // Moon icon for dark mode
                        svg {
                            class: "w-5 h-5 text-zinc-600 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path {
                                d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
                            }
                        }
                    } else {
                        // Sun icon for light mode
                        svg {
                            class: "w-5 h-5 text-zinc-600 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            circle { cx: "12", cy: "12", r: "5" }
                            line { x1: "12", y1: "1", x2: "12", y2: "3" }
                            line { x1: "12", y1: "21", x2: "12", y2: "23" }
                            line { x1: "4.22", y1: "4.22", x2: "5.64", y2: "5.64" }
                            line { x1: "18.36", y1: "18.36", x2: "19.78", y2: "19.78" }
                            line { x1: "1", y1: "12", x2: "3", y2: "12" }
                            line { x1: "21", y1: "12", x2: "23", y2: "12" }
                            line { x1: "4.22", y1: "19.78", x2: "5.64", y2: "18.36" }
                            line { x1: "18.36", y1: "5.64", x2: "19.78", y2: "4.22" }
                        }
                    }
                }
            }
        }
    }
}