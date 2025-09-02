use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_git_manager::{GitStatus, FileStatus, GitFileStatus};

#[derive(Props, Clone, PartialEq)]
pub struct FileChangesListProps {
    pub status: GitStatus,
}

#[component]
pub fn FileChangesList(props: FileChangesListProps) -> Element {
    let status = &props.status;
    
    if !status.has_changes {
        return rsx! {
            div { class: "text-center py-8 text-gray-500 dark:text-gray-400 text-sm",
                "No changes to display"
            }
        };
    }

    rsx! {
        div { class: "space-y-4",
            
            // Staged files
            if !status.staged_files.is_empty() {
                FileSection {
                    title: "Staged Changes",
                    files: status.staged_files.clone(),
                    file_status: GitFileStatus::Added,
                    icon: "📋",
                    color_class: "text-green-600 dark:text-green-400"
                }
            }
            
            // Modified files
            if !status.modified_files.is_empty() {
                FileSection {
                    title: "Modified Files",
                    files: status.modified_files.clone(),
                    file_status: GitFileStatus::Modified,
                    icon: "📝",
                    color_class: "text-orange-600 dark:text-orange-400"
                }
            }
            
            // Untracked files
            if !status.untracked_files.is_empty() {
                FileSection {
                    title: "Untracked Files",
                    files: status.untracked_files.clone(),
                    file_status: GitFileStatus::Untracked,
                    icon: "❓",
                    color_class: "text-blue-600 dark:text-blue-400"
                }
            }
        }
    }
}

#[component]
fn FileSection(
    title: String,
    files: Vec<String>,
    file_status: GitFileStatus,
    icon: String,
    color_class: String
) -> Element {
    let mut expanded = use_signal(|| true);

    rsx! {
        div { class: "border border-gray-200 dark:border-gray-700 rounded-md",
            
            // Section header
            button {
                class: "w-full px-3 py-2 flex items-center justify-between bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-t-md",
                onclick: move |_| expanded.set(!*expanded.read()),
                
                div { class: "flex items-center space-x-2",
                    span { class: "text-sm", "{icon}" }
                    span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                        "{title}"
                    }
                    span { class: format!("text-xs px-2 py-0.5 rounded-full bg-gray-200 dark:bg-gray-700 {}", color_class),
                        "{files.len()}"
                    }
                }
                
                span { class: "text-gray-400 dark:text-gray-500 text-xs",
                    if *expanded.read() { "▼" } else { "▶" }
                }
            }
            
            // File list
            if *expanded.read() {
                div { class: "divide-y divide-gray-200 dark:divide-gray-700",
                    for file in files {
                        FileItem {
                            path: file.clone(),
                            status: file_status.clone(),
                            color_class: color_class.clone()
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FileItem(path: String, status: GitFileStatus, color_class: String) -> Element {
    let file_name = path.split('/').last().unwrap_or(&path);
    let directory = if path.contains('/') {
        Some(path.rsplitn(2, '/').nth(1).unwrap_or(""))
    } else {
        None
    };

    rsx! {
        div { class: "px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-800 flex items-center justify-between",
            
            div { class: "flex items-center space-x-3 min-w-0 flex-1",
                // Status icon
                div { class: format!("flex-shrink-0 w-3 h-3 rounded-full {}", get_status_bg_color(&status)) }
                
                // File info
                div { class: "min-w-0 flex-1",
                    div { class: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate",
                        "{file_name}"
                    }
                    if let Some(dir) = directory {
                        div { class: "text-xs text-gray-500 dark:text-gray-400 truncate",
                            "{dir}/"
                        }
                    }
                }
            }
            
            // Status text
            div { class: format!("flex-shrink-0 text-xs font-medium {}", color_class),
                "{get_status_text(&status)}"
            }
        }
    }
}

#[component]
pub fn FileChangesPreview(file_path: String) -> Element {
    // This would show a diff preview of the file changes
    // For now, it's a placeholder
    rsx! {
        div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-md bg-gray-50 dark:bg-gray-800",
            div { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2",
                "Changes in {file_path}"
            }
            div { class: "text-xs text-gray-500 dark:text-gray-400",
                "Diff preview will be implemented here"
            }
        }
    }
}

#[component]
pub fn FileActionsMenu(
    file_path: String,
    status: GitFileStatus,
    on_stage: Option<EventHandler<String>>,
    on_unstage: Option<EventHandler<String>>,
    on_discard: Option<EventHandler<String>>,
    on_view_diff: Option<EventHandler<String>>
) -> Element {
    rsx! {
        div { class: "flex items-center space-x-2",
            match status {
                GitFileStatus::Modified | GitFileStatus::Untracked => rsx! {
                    if let Some(on_stage) = on_stage {
                        button {
                            class: "text-xs px-2 py-1 bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300 rounded hover:bg-green-200 dark:hover:bg-green-800",
                            onclick: move |_| on_stage.call(file_path.clone()),
                            "Stage"
                        }
                    }
                    if let Some(on_discard) = on_discard {
                        button {
                            class: "text-xs px-2 py-1 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300 rounded hover:bg-red-200 dark:hover:bg-red-800",
                            onclick: move |_| on_discard.call(file_path.clone()),
                            "Discard"
                        }
                    }
                },
                GitFileStatus::Added => rsx! {
                    if let Some(on_unstage) = on_unstage {
                        button {
                            class: "text-xs px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-700",
                            onclick: move |_| on_unstage.call(file_path.clone()),
                            "Unstage"
                        }
                    }
                },
                _ => rsx! { div {} }
            }
            
            if let Some(on_view_diff) = on_view_diff {
                button {
                    class: "text-xs px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded hover:bg-blue-200 dark:hover:bg-blue-800",
                    onclick: move |_| on_view_diff.call(file_path.clone()),
                    "Diff"
                }
            }
        }
    }
}

fn get_status_text(status: &GitFileStatus) -> &'static str {
    match status {
        GitFileStatus::Untracked => "New",
        GitFileStatus::Modified => "Modified",
        GitFileStatus::Added => "Staged",
        GitFileStatus::Deleted => "Deleted",
        GitFileStatus::Renamed => "Renamed",
        GitFileStatus::Copied => "Copied",
        GitFileStatus::UpdatedButUnmerged => "Conflict",
        GitFileStatus::Ignored => "Ignored",
    }
}

fn get_status_bg_color(status: &GitFileStatus) -> &'static str {
    match status {
        GitFileStatus::Untracked => "bg-blue-500",
        GitFileStatus::Modified => "bg-orange-500",
        GitFileStatus::Added => "bg-green-500",
        GitFileStatus::Deleted => "bg-red-500",
        GitFileStatus::Renamed => "bg-purple-500",
        GitFileStatus::Copied => "bg-indigo-500",
        GitFileStatus::UpdatedButUnmerged => "bg-red-600",
        GitFileStatus::Ignored => "bg-gray-400",
    }
}