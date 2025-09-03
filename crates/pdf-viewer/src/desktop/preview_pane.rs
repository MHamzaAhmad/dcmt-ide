use dioxus::prelude::*;
use dioxus_signals::{Readable, Signal};
use dioxus_hooks::{use_signal, use_effect};
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

#[cfg(feature = "native-git")]
use latex_ide_git_manager::{SessionManager, GitRepository};

/// Desktop PDF preview pane
#[component]
pub fn DesktopPreviewPane(
    #[props(default)] session_manager: Option<Signal<Option<String>>>,
) -> Element {
    let mut available_versions = use_signal(|| Vec::<String>::new());
    let mut current_version = use_signal(|| None::<String>);
    
    // Load available versions when session manager is ready (for desktop, this would be direct access)
    #[cfg(feature = "native-git")]
    use_effect(move || {
        if session_manager.is_some() {
            // On desktop, we could directly access the SessionManager
            // For now, we'll use a placeholder
            tracing::info!("Desktop version management would be implemented here with direct Git access");
        }
    });
    
    #[cfg(not(feature = "native-git"))]
    use_effect(move || {
        // Fallback for when git features are not available
        tracing::info!("Git features not available in this build");
    });
    rsx! {
        div { class: "h-full bg-zinc-50 dark:bg-zinc-950 border-l border-zinc-200 dark:border-zinc-800",
            
            // PDF controls
            div { class: "h-12 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between px-4",
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
                
                div { class: "flex items-center space-x-2 text-sm text-zinc-600 dark:text-zinc-400",
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
                    
                    // Version controls for desktop
                    if !available_versions.read().is_empty() {
                        div {
                            class: "flex items-center gap-2 ml-4 border-l border-zinc-200 dark:border-zinc-700 pl-4",
                            span {
                                class: "text-sm text-zinc-600 dark:text-zinc-400",
                                "Version:"
                            }
                            
                            if let Some(version) = current_version.read().as_ref() {
                                span {
                                    class: "text-sm font-mono text-blue-600 dark:text-blue-400",
                                    "{version}"
                                }
                            }
                            
                            Dropdown {
                                items: available_versions.read().iter().map(|version| {
                                    DropdownItem::new(version.clone(), version.clone())
                                        .with_description(format!("PDF version {}", version))
                                }).collect(),
                                selected: current_version.read().clone(),
                                onselect: move |version: String| {
                                    current_version.set(Some(version.clone()));
                                    tracing::info!("Desktop version rollback to {} would be implemented here", version);
                                    // Desktop would use direct SessionManager access
                                },
                                placeholder: "Select version".to_string(),
                                size: Some(DropdownSize::Small),
                                variant: Some(DropdownVariant::Default),
                            }
                        }
                    }
                }
            }
            
            // PDF viewer
            div { class: "flex-1 overflow-auto p-4",
                div { class: "bg-white shadow-lg mx-auto",
                    style: "width: 210mm; min-height: 297mm;",
                    
                    div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
                        "PDF Preview\n(Compile document to see output)"
                    }
                }
            }
        }
    }
}