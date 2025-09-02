use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use crate::{ProjectManager, FileItem};
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

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
                    current_files.set(files);
                    loading.set(false);
                    tracing::info!("Successfully loaded {} files", current_files.read().len());
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
        div { class: "h-full overflow-auto p-2 flex flex-col",
            WebFileTreeHeader {
                project_manager,
                loading,
                error_message,
                current_files,
            }
            
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

/// File tree header with project info and refresh button
#[component]
fn WebFileTreeHeader(
    project_manager: Signal<ProjectManager>,
    loading: Signal<bool>,
    error_message: Signal<Option<String>>,
    current_files: Signal<Vec<FileItem>>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between mb-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300",
                if let Some(project) = project_manager.read().get_current_project() {
                    "{project.name}"
                } else {
                    "Workspace Files"
                }
            }
            
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Small,
                disabled: *loading.read(),
                onclick: move |_| {
                    refresh_file_list(loading, error_message, current_files);
                },
                if *loading.read() { "🔄 Loading..." } else { "🔄 Refresh" }
            }
        }
        
        // Main file indicator
        if let Some(main_file) = project_manager.read().get_current_project()
            .and_then(|p| p.main_file.as_ref()) {
            div { class: "p-2 mb-2 text-sm text-blue-600 bg-blue-50 dark:bg-blue-900 dark:text-blue-200 rounded",
                "📝 Main: {main_file.display()}"
            }
        }
        
        // Error display
        if let Some(error) = error_message.read().as_ref() {
            div { class: "p-2 mb-2 text-sm text-red-600 bg-red-50 dark:bg-red-900 dark:text-red-200 rounded",
                "{error}"
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| error_message.set(None),
                    class: "ml-2 text-xs",
                    "✕"
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
        div { class: "flex-1 space-y-1",
            if *loading.read() {
                div { class: "text-center text-gray-500 py-8",
                    div { class: "inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-gray-500 mb-2" }
                    div { "Loading workspace..." }
                }
            } else if current_files.read().is_empty() {
                div { class: "text-center text-gray-500 py-8",
                    div { class: "mb-2", "📁 No files found" }
                    div { class: "text-sm", "Upload files or create new ones to get started" }
                }
            } else {
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

/// Action buttons for file operations
#[component]
fn WebFileTreeActions(
    current_files: Signal<Vec<FileItem>>,
    error_message: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "mt-2 pt-2 border-t border-gray-200 dark:border-gray-700",
            div { class: "grid grid-cols-2 gap-2",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        create_new_latex_file(current_files, error_message);
                    },
                    "📄 New .tex"
                }
                
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        create_new_bibliography_file(current_files, error_message);
                    },
                    "📚 New .bib"
                }
                
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        upload_file_from_local(current_files, error_message);
                    },
                    "📤 Upload"
                }
                
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        // TODO: Implement workspace download
                        tracing::info!("Download workspace clicked");
                    },
                    "📥 Download"
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
    let file_icon = file_item.icon();
    let file_name = file_item.name.clone();
    let file_size = file_item.size;
    let is_file = file_item.is_file();
    
    rsx! {
        div { 
            class: if is_selected {
                "flex items-center space-x-2 px-2 py-1 text-sm bg-blue-100 dark:bg-blue-800 text-blue-900 dark:text-blue-100 rounded cursor-pointer transition-colors group"
            } else {
                "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer transition-colors group"
            },
            
            div {
                class: "flex items-center space-x-2 flex-1",
                onclick: move |_| {
                    onclick.call(file_path_for_click.clone());
                },
                
                span { "{file_icon}" }
                span { 
                    title: "{file_path_display}", 
                    class: "truncate flex-1",
                    "{file_name}" 
                }
                
                // Show file size for files
                if is_file {
                    if let Some(size) = file_size {
                        span { class: "text-xs text-gray-400 ml-auto",
                            "{format_file_size(size)}"
                        }
                    }
                }
            }
            
            // Delete button (appears on hover)
            div { class: "opacity-0 group-hover:opacity-100 transition-opacity",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        delete_file_item(file_name.clone(), current_files, error_message);
                    },
                    class: "text-red-500 hover:text-red-700 p-1",
                    "🗑️"
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