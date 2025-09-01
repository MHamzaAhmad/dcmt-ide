use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable};
use dioxus_hooks::use_signal;
use crate::{ProjectManager, FileItem};
use latex_ide_ui::*;
use latex_ide_ui::button::{ButtonVariant, ButtonSize};

#[component]
pub fn WebFileTree(project_manager: Signal<ProjectManager>) -> Element {
    // Default files for the demo
    let files = use_signal(|| vec![
        ("document.tex".to_string(), true),
        ("figures/".to_string(), false),
        ("references.bib".to_string(), true),
        ("styles.sty".to_string(), true),
    ]);
    
    rsx! {
        div { class: "h-full overflow-auto p-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                "Project Files"
            }
            
            div { class: "space-y-1",
                {
                    files.read().iter().enumerate().map(|(i, (name, is_file))| {
                        let name = name.clone();
                        let is_file = *is_file;
                        rsx! {
                            WebFileItem { 
                                key: "{i}",
                                name: name.clone(),
                                is_file: is_file,
                                onclick: move |_| {
                                    tracing::info!("Clicked file: {}", name);
                                    // TODO: Integrate with project manager
                                }
                            }
                        }
                    })
                }
            }
            
            // Add file/folder buttons
            div { class: "mt-4 pt-4 border-t border-gray-200 dark:border-gray-700",
                div { class: "flex space-x-2",
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Create new file
                            tracing::info!("New file button clicked");
                        },
                        "📄 New File"
                    }
                    
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Small,
                        onclick: move |_| {
                            // Upload file
                            tracing::info!("Upload button clicked");
                        },
                        "📤 Upload"
                    }
                }
            }
        }
    }
}

#[component]
pub fn WebFileItem(name: String, is_file: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let icon = if is_file { "📄" } else { "📁" };
    
    rsx! {
        div { 
            class: "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer transition-colors",
            onclick: move |evt| onclick.call(evt),
            
            span { "{icon}" }
            span { "{name}" }
        }
    }
}

/// Advanced file tree component using the file manager data structures
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