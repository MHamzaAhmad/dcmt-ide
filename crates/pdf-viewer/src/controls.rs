use latex_ide_ui::*;
use latex_ide_ui::button::ButtonVariant;
use crate::PDFDocument;

#[cfg(target_arch = "wasm32")]
use {
    dioxus_hooks::use_effect,
    wasm_bindgen_futures::spawn_local,
};

#[derive(Clone, Debug)]
pub struct VersionInfo {
    pub version: String,
    pub timestamp: String,
    pub commit_id: String,
}

#[component]
pub fn PDFControls(
    #[props(into)] document: Signal<Option<PDFDocument>>,
    #[props(default)] current_version: Option<Signal<Option<String>>>,
    #[props(default)] available_versions: Option<Signal<Vec<String>>>,
    #[props(default)] on_version_select: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-2 p-3 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900",
            
            if let Some(doc) = &*document.read() {
                // Navigation controls
                Button {
                    variant: ButtonVariant::Secondary,
                    disabled: doc.current_page == 0,
                    onclick: move |_| {
                        if let Some(doc) = document.write().as_mut() {
                            doc.prev_page();
                        }
                    },
                    "◀"
                }
                
                span {
                    class: "mx-2 text-sm text-zinc-600 dark:text-zinc-400",
                    "Page {doc.current_page + 1} of {doc.total_pages}"
                }
                
                Button {
                    variant: ButtonVariant::Secondary,
                    disabled: doc.current_page + 1 >= doc.total_pages,
                    onclick: move |_| {
                        if let Some(doc) = document.write().as_mut() {
                            doc.next_page();
                        }
                    },
                    "▶"
                }
                
                // Zoom controls
                div {
                    class: "flex items-center gap-1 ml-4",
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            if let Some(doc) = document.write().as_mut() {
                                doc.zoom_out();
                            }
                        },
                        "−"
                    }
                    
                    span {
                        class: "mx-2 text-sm text-zinc-600 dark:text-zinc-400 min-w-[50px] text-center",
                        "{(doc.zoom * 100.0) as i32}%"
                    }
                    
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            if let Some(doc) = document.write().as_mut() {
                                doc.zoom_in();
                            }
                        },
                        "+"
                    }
                }
                
                // Fit controls
                div {
                    class: "flex items-center gap-1 ml-4",
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            if let Some(doc) = document.write().as_mut() {
                                doc.fit_width(800.0); // Default container width
                            }
                        },
                        "Fit Width"
                    }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            if let Some(doc) = document.write().as_mut() {
                                doc.fit_height(600.0); // Default container height
                            }
                        },
                        "Fit Height"
                    }
                }
                
                // Version controls
                if let Some(versions) = &available_versions {
                    if !versions.read().is_empty() {
                        div {
                            class: "flex items-center gap-2 ml-4 border-l border-zinc-200 dark:border-zinc-700 pl-4",
                            span {
                                class: "text-sm text-zinc-600 dark:text-zinc-400",
                                "Version:"
                            }
                            
                            // Current version display
                            if let Some(current_ver) = &current_version {
                                if let Some(version) = current_ver.read().as_ref() {
                                    span {
                                        class: "text-sm font-mono text-blue-600 dark:text-blue-400",
                                        "{version}"
                                    }
                                }
                            }
                            
                            // Version dropdown
                            Dropdown {
                                items: versions.read().iter().map(|version| {
                                    DropdownItem::new(version.clone(), version.clone())
                                        .with_description(format!("PDF version {}", version))
                                }).collect(),
                                selected: current_version.and_then(|cv| cv.read().clone()),
                                onselect: move |version: String| {
                                    if let Some(handler) = &on_version_select {
                                        handler.call(version);
                                    }
                                },
                                placeholder: "Select version".to_string(),
                                size: Some(DropdownSize::Small),
                                variant: Some(DropdownVariant::Default),
                            }
                        }
                    }
                }
            } else {
                span {
                    class: "text-zinc-500 dark:text-zinc-400",
                    "No PDF loaded"
                }
            }
        }
    }
}