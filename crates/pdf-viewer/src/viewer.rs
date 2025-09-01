use latex_ide_ui::*;
use crate::{PDFDocument, PDFControls, PDFRenderer};

#[component]
pub fn PDFViewer(
    #[props(into)] document: Signal<Option<PDFDocument>>,
    #[props(default = String::from("100%"))] width: String,
    #[props(default = String::from("600px"))] height: String,
) -> Element {
    rsx! {
        div {
            class: "pdf-viewer flex flex-col",
            style: "width: {width}; height: {height};",
            
            // PDF Controls
            PDFControls {
                document: document
            }
            
            // PDF Content
            div {
                class: "flex-1 overflow-auto bg-gray-100 dark:bg-gray-800",
                PDFRenderer {
                    document: document
                }
            }
        }
    }
}