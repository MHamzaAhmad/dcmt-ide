use latex_ide_ui::*;
use latex_ide_ui::button::ButtonVariant;
use crate::PDFDocument;

#[component]
pub fn PDFControls(
    #[props(into)] document: Signal<Option<PDFDocument>>,
) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-2 p-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900",
            
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
                    class: "mx-2 text-sm text-gray-600 dark:text-gray-400",
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
                        class: "mx-2 text-sm text-gray-600 dark:text-gray-400 min-w-[50px] text-center",
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
            } else {
                span {
                    class: "text-gray-500 dark:text-gray-400",
                    "No PDF loaded"
                }
            }
        }
    }
}