use latex_ide_ui::*;
use crate::PDFDocument;
use std::path::PathBuf;
use base64::Engine;

#[component]
pub fn PDFRenderer(
    #[props(into)] document: Signal<Option<PDFDocument>>,
) -> Element {
    let render_task = use_resource(move || {
        async move {
            if let Some(doc) = document.read().as_ref() {
                let path = doc.path.clone();
                let page = doc.current_page;
                let zoom = doc.zoom;
                render_pdf_page(&path, page, zoom).await
            } else {
                Ok(None)
            }
        }
    });

    rsx! {
        div {
            class: "flex items-center justify-center w-full h-full",
            
            match render_task.read().as_ref() {
                Some(Ok(Some(page_data))) => rsx! {
                    div {
                        class: "pdf-page shadow-lg",
                        style: "max-width: 100%; max-height: 100%;",
                        img {
                            src: "data:image/png;base64,{base64::engine::general_purpose::STANDARD.encode(page_data)}",
                            alt: "PDF Page",
                            class: "max-w-full max-h-full object-contain"
                        }
                    }
                },
                Some(Ok(None)) => rsx! {
                    div {
                        class: "flex items-center justify-center text-gray-500 dark:text-gray-400",
                        "No PDF loaded"
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "flex items-center justify-center text-red-500",
                        "Error loading PDF: {e}"
                    }
                },
                None => rsx! {
                    div {
                        class: "flex items-center justify-center text-gray-500 dark:text-gray-400",
                        "Loading..."
                    }
                }
            }
        }
    }
}

async fn render_pdf_page(
    _path: &PathBuf, 
    _page: usize, 
    _zoom: f32
) -> anyhow::Result<Option<Vec<u8>>> {
    // TODO: Implement actual PDF rendering using pdfium-render
    // For now, return None to avoid compilation errors
    Ok(None)
}