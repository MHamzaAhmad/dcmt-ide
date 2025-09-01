use dioxus::prelude::*;
use dioxus_signals::Readable;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

/// Desktop PDF preview pane
#[component]
pub fn DesktopPreviewPane() -> Element {
    rsx! {
        div { class: "h-full bg-gray-50 dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700",
            
            // PDF controls
            div { class: "h-12 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4",
                div { class: "flex items-center space-x-2",
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Compile document
                            tracing::info!("Compile button clicked");
                        },
                        "Compile"
                    }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Refresh PDF
                            tracing::info!("Refresh button clicked");
                        },
                        "🔄"
                    }
                }
                
                div { class: "flex items-center space-x-2 text-sm text-gray-600 dark:text-gray-400",
                    span { "Page 1 of 1" }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Zoom out
                        },
                        "−"
                    }
                    
                    span { "100%" }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Zoom in
                        },
                        "+"
                    }
                }
            }
            
            // PDF viewer
            div { class: "flex-1 overflow-auto p-4",
                div { class: "bg-white shadow-lg mx-auto",
                    style: "width: 210mm; min-height: 297mm;",
                    
                    div { class: "h-full flex items-center justify-center text-gray-500",
                        "PDF Preview\n(Compile document to see output)"
                    }
                }
            }
        }
    }
}