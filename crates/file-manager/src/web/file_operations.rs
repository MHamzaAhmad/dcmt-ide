use crate::FileItem;
use super::transport::{upload_file, download_file, delete_file, list_files};
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen::closure::Closure,
    web_sys::{FileReader, HtmlInputElement, window},
    js_sys::Uint8Array,
    wasm_bindgen_futures::spawn_local,
    gloo_timers,
};

/// File upload result
pub type UploadResult = Result<(), String>;

/// File download result containing the file content
pub type DownloadResult = Result<Vec<u8>, String>;

/// File list result containing FileItem vector
pub type FileListResult = Result<Vec<FileItem>, String>;

/// Real file upload implementation using FileReader API
#[cfg(target_arch = "wasm32")]
pub async fn upload_file_from_browser(file: web_sys::File) -> UploadResult {
    let file_name = file.name();
    tracing::info!("Starting upload for file: {}", file_name);
    
    // Create FileReader to read file content
    let file_reader = FileReader::new().map_err(|e| format!("Failed to create FileReader: {:?}", e))?;
    let file_reader_clone = file_reader.clone();
    
    // Set up promise-like behavior using Rc<RefCell<>>
    let content_result = std::rc::Rc::new(std::cell::RefCell::new(None::<Result<Vec<u8>, String>>));
    let content_result_clone = content_result.clone();
    
    // Set up onload handler
    let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
        match file_reader_clone.result() {
            Ok(result) => {
                match result.dyn_into::<js_sys::ArrayBuffer>() {
                    Ok(array_buffer) => {
                        let uint8_array = Uint8Array::new(&array_buffer);
                        let content = uint8_array.to_vec();
                        *content_result_clone.borrow_mut() = Some(Ok(content));
                    }
                    Err(_) => {
                        *content_result_clone.borrow_mut() = Some(Err("Failed to convert result to ArrayBuffer".to_string()));
                    }
                }
            }
            Err(_) => {
                *content_result_clone.borrow_mut() = Some(Err("FileReader error".to_string()));
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    
    // Set up onerror handler  
    let error_result_clone = content_result.clone();
    let onerror = Closure::wrap(Box::new(move |_: web_sys::Event| {
        *error_result_clone.borrow_mut() = Some(Err("Failed to read file".to_string()));
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    file_reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    
    // Start reading the file as ArrayBuffer
    file_reader.read_as_array_buffer(&file).map_err(|e| format!("Failed to start reading file: {:?}", e))?;
    
    // Wait for file reading to complete
    let mut attempts = 0;
    while attempts < 100 {
        if let Some(result) = content_result.borrow().as_ref() {
            let content = result.clone()?;
            
            // Prevent closures from being dropped
            onload.forget();
            onerror.forget();
            
            // Upload the file content
            return upload_file(&file_name, content).await;
        }
        gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;
        attempts += 1;
    }
    
    // Prevent closures from being dropped
    onload.forget();
    onerror.forget();
    
    Err("Timeout reading file".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn upload_file_from_browser(_file: ()) -> UploadResult {
    Err("File upload not available on non-WASM platforms".to_string())
}

/// Create a file input element and trigger file selection
#[cfg(target_arch = "wasm32")]
pub fn create_file_input<F>(callback: F) -> Result<(), String> 
where 
    F: FnMut(web_sys::File) + 'static,
{
    let document = window()
        .ok_or("No window object")?
        .document()
        .ok_or("No document object")?;
        
    let input: HtmlInputElement = document
        .create_element("input")
        .map_err(|e| format!("Failed to create input element: {:?}", e))?
        .dyn_into()
        .map_err(|_| "Failed to cast to HtmlInputElement")?;
    
    input.set_type("file");
    input.set_accept(".tex,.txt,.md,.bib,.cls,.sty,.pdf");
    
    let input_clone = input.clone();
    let callback_ref = std::rc::Rc::new(std::cell::RefCell::new(callback));
    let callback_clone = callback_ref.clone();
    
    let onchange = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if let Some(file_list) = input_clone.files() {
            if let Some(file) = file_list.get(0) {
                if let Ok(mut cb) = callback_clone.try_borrow_mut() {
                    cb(file);
                }
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    input.click();
    onchange.forget();
    
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_file_input<F>(_callback: F) -> Result<(), String> 
where 
    F: Fn(()) + 'static,
{
    Err("File input not available on non-WASM platforms".to_string())
}

/// Download file content from server
pub async fn download_file_content(file_name: &str) -> DownloadResult {
    download_file(file_name).await
}

/// Delete file on server
pub async fn delete_file_on_server(file_name: &str) -> Result<(), String> {
    delete_file(file_name).await
}

/// Fetch file list from server and convert to FileItem format
pub async fn fetch_file_list() -> FileListResult {
    let file_infos = list_files().await?;
    
    let mut items = Vec::new();
    for file_info in file_infos {
        let path = PathBuf::from(&file_info.path);
        let item = if file_info.is_directory {
            FileItem::new_directory(file_info.name, path)
        } else {
            let mut item = FileItem::new_file(file_info.name, path);
            item.size = file_info.size;
            item
        };
        items.push(item);
    }
    
    Ok(items)
}

/// Create a new LaTeX file with template content
pub async fn create_latex_file(file_name: &str) -> Result<(), String> {
    let template_content = r#"\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{amsfonts}
\usepackage{amssymb}

\title{New LaTeX Document}
\author{Author Name}
\date{\today}

\begin{document}

\maketitle

\section{Introduction}

Write your content here.

\section{Conclusion}

Your conclusions go here.

\end{document}
"#;

    upload_file(file_name, template_content.as_bytes().to_vec()).await
}

/// Create a new bibliography file
pub async fn create_bibliography_file(file_name: &str) -> Result<(), String> {
    let bib_content = r#"@article{example2023,
    title={Example Article Title},
    author={Author, First and Author, Second},
    journal={Journal Name},
    volume={1},
    number={1},
    pages={1--10},
    year={2023},
    publisher={Publisher Name}
}

@book{example_book2023,
    title={Example Book Title},
    author={Book Author},
    year={2023},
    publisher={Publisher Name},
    address={City, Country}
}
"#;

    upload_file(file_name, bib_content.as_bytes().to_vec()).await
}

/// Format file size in human-readable format
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Validate file name for LaTeX files
pub fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("File name cannot be empty".to_string());
    }
    
    if name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("File name contains invalid characters".to_string());
    }
    
    if name.len() > 255 {
        return Err("File name is too long".to_string());
    }
    
    Ok(())
}

/// Get file extension
pub fn get_file_extension(name: &str) -> Option<&str> {
    std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
}

/// Check if file is a LaTeX file
pub fn is_latex_file(name: &str) -> bool {
    matches!(get_file_extension(name), Some("tex") | Some("cls") | Some("sty"))
}

/// Check if file is a bibliography file
pub fn is_bibliography_file(name: &str) -> bool {
    matches!(get_file_extension(name), Some("bib"))
}

/// Check if file is a PDF file
pub fn is_pdf_file(name: &str) -> bool {
    matches!(get_file_extension(name), Some("pdf"))
}