use dioxus::prelude::*;
use dioxus_signals::{Readable, Signal};
use dioxus_hooks::{use_signal, use_effect};
use crate::{ProjectManager, FileItem, FileSystemBackend, NativeFileSystem};
use crate::desktop::pick_workspace_directory;
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

/// Desktop file tree component with real file system integration
#[component]
pub fn DesktopFileTree(
    project_manager: Signal<ProjectManager>,
    onfile_select: EventHandler<String>
) -> Element {
    let fs_backend = use_signal(|| NativeFileSystem::new());
    let mut current_files = use_signal(|| Vec::<FileItem>::new());
    let mut error_message = use_signal(|| None::<String>);
    
    // Update file tree when project changes
    use_effect(move || {
        if let Some(workspace_root) = project_manager.read().get_workspace_root() {
            match fs_backend.read().read_dir(workspace_root) {
                Ok(files) => {
                    current_files.set(files);
                    error_message.set(None);
                }
                Err(e) => {
                    error_message.set(Some(format!("Error reading directory: {}", e)));
                    current_files.set(Vec::new());
                }
            }
        } else {
            current_files.set(Vec::new());
            error_message.set(None);
        }
    });
    
    rsx! {
        div { class: "h-full overflow-auto p-2 flex flex-col",
            // Header with project name and controls
            div { class: "flex items-center justify-between mb-2",
                div { class: "text-sm font-medium text-gray-700 dark:text-gray-300",
                    if let Some(project) = project_manager.read().get_current_project() {
                        "{project.name}"
                    } else {
                        "No Project Open"
                    }
                }
                
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Small,
                    onclick: move |_| {
                        if let Some(workspace_path) = pick_workspace_directory() {
                            tracing::info!("Selected workspace: {:?}", workspace_path);
                            let mut pm = project_manager.write();
                            if let Err(e) = pm.open_workspace(workspace_path) {
                                tracing::error!("Failed to open workspace: {}", e);
                                error_message.set(Some(format!("Failed to open workspace: {}", e)));
                            }
                        }
                    },
                    "📁"
                }
            }
            
            // Error display
            if let Some(error) = error_message.read().as_ref() {
                div { class: "p-2 mb-2 text-sm text-red-600 bg-red-50 dark:bg-red-900 dark:text-red-200 rounded",
                    "{error}"
                }
            }
            
            // Main file indicator
            if let Some(main_file) = project_manager.read().get_current_project()
                .and_then(|p| p.main_file.as_ref()) {
                div { class: "p-2 mb-2 text-sm text-blue-600 bg-blue-50 dark:bg-blue-900 dark:text-blue-200 rounded",
                    "📝 Main: {main_file.display()}"
                }
            }
            
            // File tree
            div { class: "flex-1 space-y-1",
                if current_files.read().is_empty() {
                    if project_manager.read().get_current_project().is_some() {
                        div { class: "text-center text-gray-500 py-4",
                            "No files found"
                        }
                    } else {
                        div { class: "text-center text-gray-500 py-8",
                            div { class: "mb-2", "No project open" }
                            Button {
                                variant: ButtonVariant::Primary,
                                size: ButtonSize::Small,
                                onclick: move |_| {
                                    if let Some(workspace_path) = pick_workspace_directory() {
                                        tracing::info!("Selected workspace: {:?}", workspace_path);
                                        let mut pm = project_manager.write();
                                        if let Err(e) = pm.open_workspace(workspace_path) {
                                            tracing::error!("Failed to open workspace: {}", e);
                                            error_message.set(Some(format!("Failed to open workspace: {}", e)));
                                        }
                                    }
                                },
                                "Open Folder"
                            }
                        }
                    }
                } else {
                    for (index, file) in current_files.read().iter().enumerate() {
                        DesktopFileItem {
                            key: "{index}",
                            file_item: file.clone(),
                            onclick: move |file_path: String| {
                                tracing::info!("Selected file: {}", file_path);
                                onfile_select.call(file_path);
                            }
                        }
                    }
                }
            }
            
            // Action buttons at bottom
            if project_manager.read().get_current_project().is_some() {
                div { class: "mt-2 pt-2 border-t border-gray-200 dark:border-gray-700",
                    div { class: "flex space-x-2",
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Small,
                            onclick: move |_| {
                                // Create new LaTeX file with template
                                if let Some(workspace_path) = project_manager.read().get_workspace_root() {
                                    let file_name = "new_document.tex"; // In a real implementation, this would be from a dialog
                                    let new_file_path = workspace_path.join(file_name);
                                    
                                    // Create basic LaTeX template
                                    let latex_template = r#"\documentclass{article}
\usepackage[utf8]{inputenc}
\title{New Document}
\author{Author}
\date{\today}

\begin{document}
\maketitle

\section{Introduction}
Your content here.

\end{document}
"#;
                                    
                                    match fs_backend.read().write_file(&new_file_path, latex_template) {
                                        Ok(_) => {
                                            tracing::info!("Created new LaTeX file: {:?}", new_file_path);
                                            // Refresh file tree
                                            if let Some(workspace_root) = project_manager.read().get_workspace_root() {
                                                if let Ok(files) = fs_backend.read().read_dir(workspace_root) {
                                                    current_files.set(files);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to create new file: {}", e);
                                            error_message.set(Some(format!("Failed to create file: {}", e)));
                                        }
                                    }
                                }
                            },
                            "📄 New File"
                        }
                        
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Small,
                            onclick: move |_| {
                                // Create new folder
                                if let Some(workspace_path) = project_manager.read().get_workspace_root() {
                                    let folder_name = "new_folder"; // In a real implementation, this would be from a dialog
                                    let new_folder_path = workspace_path.join(folder_name);
                                    
                                    match fs_backend.read().create_dir(&new_folder_path) {
                                        Ok(_) => {
                                            tracing::info!("Created new folder: {:?}", new_folder_path);
                                            // Refresh file tree
                                            if let Some(workspace_root) = project_manager.read().get_workspace_root() {
                                                if let Ok(files) = fs_backend.read().read_dir(workspace_root) {
                                                    current_files.set(files);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to create new folder: {}", e);
                                            error_message.set(Some(format!("Failed to create folder: {}", e)));
                                        }
                                    }
                                }
                            },
                            "📁 New Folder"
                        }
                    }
                }
            }
        }
    }
}

/// Enhanced desktop file item component
#[component]
fn DesktopFileItem(
    file_item: FileItem,
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
                "flex items-center space-x-2 px-2 py-1 text-sm bg-blue-100 dark:bg-blue-800 text-blue-900 dark:text-blue-100 rounded cursor-pointer"
            } else {
                "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer transition-colors"
            },
            onclick: move |_| {
                onclick.call(file_path_for_click.clone());
            },
            
            span { "{file_icon}" }
            span { title: "{file_path_display}", "{file_name}" }
            
            // Show file size for files
            if is_file {
                if let Some(size) = file_size {
                    span { class: "ml-auto text-xs text-gray-400",
                        "{format_file_size(size)}"
                    }
                }
            }
        }
    }
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
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