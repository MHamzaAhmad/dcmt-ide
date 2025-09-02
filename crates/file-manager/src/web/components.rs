use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use crate::{ProjectManager, FileItem};
use latex_ide_ui::*;

use super::file_operations::{
    fetch_file_list, format_file_size
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// Main file tree component for web platform
#[component]
pub fn WebFileTree(
    project_manager: Signal<ProjectManager>,
    onfile_select: Option<EventHandler<String>>
) -> Element {
    let mut current_files = use_signal(|| Vec::<FileItem>::new());
    let mut error_message = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    
    // Load files from workspace via transport
    use_effect(move || {
        loading.set(true);
        error_message.set(None);
        
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            match fetch_file_list().await {
                Ok(files) => {
                    current_files.set(files.clone());
                    loading.set(false);
                    tracing::info!("Successfully loaded {} files", files.len());
                    
                    // Auto-select the first .tex file if available
                    if let Some(onfile_select) = &onfile_select {
                        if let Some(tex_file) = files.iter().find(|f| f.name.ends_with(".tex")) {
                            tracing::info!("Auto-selecting first .tex file: {}", tex_file.name);
                            onfile_select.call(tex_file.name.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load files: {}", e);
                    loading.set(false);
                    error_message.set(Some(format!("Failed to load files: {}", e)));
                }
            }
        });
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            loading.set(false);
            error_message.set(Some("File operations not available on non-WASM platforms".to_string()));
        }
    });
    
    rsx! {
        div { class: "h-full flex flex-col",
            // Simple header with title and refresh
            div { 
                class: "h-10 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between px-3 bg-white dark:bg-zinc-900",
                
                span { 
                    class: "text-xs font-medium text-zinc-700 dark:text-zinc-300 uppercase tracking-wider",
                    "Explorer"
                }
                
                // Refresh button
                button {
                    class: "p-1 rounded hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors",
                    onclick: move |_| {
                        refresh_file_list(loading, error_message, current_files);
                    },
                    disabled: *loading.read(),
                    title: "Refresh Files",
                    
                    svg {
                        class: if *loading.read() { "w-3.5 h-3.5 text-zinc-400 animate-spin" } else { "w-3.5 h-3.5 text-zinc-600 dark:text-zinc-400" },
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" }
                    }
                }
            }
            
            // File tree content
            div { class: "flex-1 overflow-auto",
                WebFileTreeContent {
                    current_files,
                    loading,
                    error_message,
                    onfile_select,
                }
                
                WebFileTreeActions {
                    current_files,
                    error_message,
                }
            }
        }
    }
}


/// Main content area displaying file list
#[component]
fn WebFileTreeContent(
    current_files: Signal<Vec<FileItem>>,
    loading: Signal<bool>,
    error_message: Signal<Option<String>>,
    onfile_select: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div { class: "flex-1 px-2 py-3",
            if *loading.read() {
                div { class: "flex flex-col items-center justify-center py-8",
                    div { class: "w-6 h-6 border-2 border-zinc-300 dark:border-zinc-600 border-t-transparent rounded-full animate-spin mb-3" }
                    div { class: "text-sm text-zinc-500 dark:text-zinc-400", "Loading workspace..." }
                }
            } else if current_files.read().is_empty() {
                div { class: "flex flex-col items-center justify-center py-8 text-zinc-500 dark:text-zinc-400",
                    svg {
                        class: "w-12 h-12 mb-3 text-zinc-300 dark:text-zinc-700",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
                    }
                    div { class: "text-sm font-medium mb-1", "No files found" }
                    div { class: "text-xs", "Create or upload files to get started" }
                }
            } else {
                div { class: "space-y-0.5",
                    for (index, file) in current_files.read().iter().enumerate() {
                        WebFileItem {
                            key: "{index}",
                            file_item: file.clone(),
                            error_message,
                            current_files,
                            onclick: move |file_path: String| {
                                tracing::info!("Selected file: {}", file_path);
                                if let Some(handler) = &onfile_select {
                                    handler.call(file_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Action buttons for file operations
#[component]
fn WebFileTreeActions(
    current_files: Signal<Vec<FileItem>>,
    error_message: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "p-3 border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900",
            div { class: "grid grid-cols-2 gap-2",
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        create_new_latex_file(current_files, error_message);
                    },
                    "New .tex"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        create_new_bibliography_file(current_files, error_message);
                    },
                    "New .bib"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        upload_file_from_local(current_files, error_message);
                    },
                    "Upload"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        // TODO: Implement workspace download
                        tracing::info!("Download workspace clicked");
                    },
                    "Download"
                }
            }
        }
    }
}

/// Individual file item component
#[component]
pub fn WebFileItem(
    file_item: FileItem,
    error_message: Signal<Option<String>>,
    current_files: Signal<Vec<FileItem>>,
    onclick: EventHandler<String>
) -> Element {
    let is_selected = false; // TODO: Add selection state management
    let file_path_display = file_item.path.display().to_string();
    let file_path_for_click = file_path_display.clone();
    let file_name = file_item.name.clone();
    let file_size = file_item.size;
    let is_file = file_item.is_file();
    let is_tex = file_name.ends_with(".tex");
    let is_bib = file_name.ends_with(".bib");
    
    rsx! {
        div { 
            class: if is_selected {
                "flex items-center px-3 py-1.5 text-sm bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 rounded-md cursor-pointer transition-colors group"
            } else {
                "flex items-center px-3 py-1.5 text-sm text-zinc-700 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-md cursor-pointer transition-colors group"
            },
            
            div {
                class: "flex items-center space-x-2 flex-1 min-w-0",
                onclick: move |_| {
                    onclick.call(file_path_for_click.clone());
                },
                
                // File icon
                if is_file {
                    if is_tex {
                        svg {
                            class: "w-4 h-4 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                            polyline { points: "14 2 14 8 20 8" }
                            line { x1: "16", y1: "13", x2: "8", y2: "13" }
                            line { x1: "16", y1: "17", x2: "8", y2: "17" }
                        }
                    } else if is_bib {
                        svg {
                            class: "w-4 h-4 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M4 19.5A2.5 2.5 0 0 1 6.5 17H20" }
                            path { d: "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" }
                        }
                    } else {
                        svg {
                            class: "w-4 h-4 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" }
                            polyline { points: "13 2 13 9 20 9" }
                        }
                    }
                } else {
                    svg {
                        class: "w-4 h-4 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
                    }
                }
                
                span { 
                    title: "{file_path_display}", 
                    class: "truncate",
                    "{file_name}" 
                }
                
                // Show file size for files
                if is_file {
                    if let Some(size) = file_size {
                        span { class: "text-xs text-zinc-400 dark:text-zinc-500 ml-auto",
                            "{format_file_size(size)}"
                        }
                    }
                }
            }
            
            // Delete button (appears on hover)
            button {
                class: "opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700 transition-all",
                onclick: move |_| {
                    delete_file_item(file_name.clone(), current_files, error_message);
                },
                
                svg {
                    class: "w-3.5 h-3.5 text-zinc-500 dark:text-zinc-400",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    view_box: "0 0 24 24",
                    polyline { points: "3 6 5 6 21 6" }
                    path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
                }
            }
        }
    }
}

/// Advanced file tree component using project manager data structures
#[component]
pub fn WebFileTreeAdvanced(
    project_manager: Signal<ProjectManager>, 
    onfile_select: EventHandler<String>
) -> Element {
    rsx! {
        div { class: "h-full overflow-auto p-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                if let Some(project) = project_manager.read().get_current_project() {
                    "{project.name}"
                } else {
                    "No Project"
                }
            }
            
            if let Some(file_tree) = project_manager.read().get_file_tree() {
                FileTreeNode {
                    item: file_tree.root.clone(),
                    project_manager: project_manager,
                    onfile_select: onfile_select,
                }
            } else {
                div { class: "text-center text-gray-500 py-8",
                    "No project loaded"
                }
            }
        }
    }
}

/// Recursive file tree node component
#[component]
fn FileTreeNode(
    item: FileItem, 
    project_manager: Signal<ProjectManager>,
    onfile_select: EventHandler<String>
) -> Element {
    let is_selected = project_manager.read().get_file_tree()
        .and_then(|tree| tree.selected_item.as_ref())
        .map(|selected| *selected == item.id)
        .unwrap_or(false);
        
    rsx! {
        div {
            div { 
                class: if is_selected {
                    "flex items-center space-x-2 px-2 py-1 text-sm bg-blue-100 dark:bg-blue-800 text-blue-900 dark:text-blue-100 rounded cursor-pointer"
                } else {
                    "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer"
                },
                onclick: move |_| {
                    tracing::info!("Clicked file: {}", item.name);
                    onfile_select.call(item.id.clone());
                },
                
                if item.is_directory() && item.children.as_ref().map_or(false, |c| !c.is_empty()) {
                    span { 
                        class: "text-xs text-gray-400",
                        if item.expanded { "▼" } else { "▶" }
                    }
                }
                
                span { "{item.icon()}" }
                span { "{item.name}" }
            }
            
            if item.expanded {
                if let Some(children) = &item.children {
                    div { class: "ml-4",
                        for child in children {
                            FileTreeNode {
                                item: child.clone(),
                                project_manager: project_manager,
                                onfile_select: onfile_select,
                            }
                        }
                    }
                }
            }
        }
    }
}

// Helper functions for file operations

fn refresh_file_list(
    mut loading: Signal<bool>,
    mut error_message: Signal<Option<String>>,
    mut current_files: Signal<Vec<FileItem>>,
) {
    tracing::info!("Refreshing file list");
    loading.set(true);
    error_message.set(None);
    
    #[cfg(target_arch = "wasm32")]
    spawn_local(async move {
        match fetch_file_list().await {
            Ok(files) => {
                current_files.set(files);
                loading.set(false);
                tracing::info!("File list refreshed successfully");
            }
            Err(e) => {
                loading.set(false);
                error_message.set(Some(format!("Refresh failed: {}", e)));
                tracing::error!("Failed to refresh file list: {}", e);
            }
        }
    });
}

// TODO: Implement create_new_latex_file when needed
#[allow(dead_code)]
fn create_new_latex_file(
    _current_files: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    tracing::info!("Create new LaTeX file functionality not yet implemented");
}

#[allow(dead_code)]
fn create_new_bibliography_file(
    _current_files: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    tracing::info!("Create new bibliography file functionality not yet implemented");
}

#[allow(dead_code)]
fn upload_file_from_local(
    _current_files: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    tracing::info!("File upload functionality not yet implemented");
}

#[allow(dead_code)]
fn delete_file_item(
    _file_name: String,
    _current_files: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    tracing::info!("File deletion functionality not yet implemented");
}