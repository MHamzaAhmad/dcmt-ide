use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_git_manager::{CommitInfo, BranchInfo};
use chrono::{DateTime, Utc};
use crate::use_version_control;

#[derive(Props, Clone, PartialEq)]
pub struct HistoryViewProps {
    pub version_control: Signal<crate::VersionControlState>,
    #[props(default)]
    pub limit: Option<usize>,
    #[props(default)]
    pub branch_filter: Option<String>,
    pub on_rollback: EventHandler<String>,
}

#[component]
pub fn HistoryView(props: HistoryViewProps) -> Element {
    let mut commits = use_signal(|| Vec::<CommitInfo>::new());
    let mut loading = use_signal(|| false);
    let mut selected_commit = use_signal(|| None::<String>);
    let mut show_commit_details = use_signal(|| false);

    // Load commit history
    use_effect(move || {
        let version_control = props.version_control.clone();
        spawn(async move {
            loading.set(true);
            if let Some(ref history_viewer) = version_control.read().history_viewer {
                match history_viewer.get_commit_history(props.branch_filter.as_deref(), props.limit) {
                    Ok(commit_history) => commits.set(commit_history),
                    Err(e) => tracing::error!("Failed to load commit history: {}", e),
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "space-y-4",
            
            // Header with filters
            div { class: "flex items-center justify-between",
                h4 { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "Commit History"
                }
                div { class: "flex items-center space-x-2",
                    BranchFilter { 
                        version_control: props.version_control,
                        current_filter: props.branch_filter.clone()
                    }
                    button {
                        class: "text-xs px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded hover:bg-blue-200 dark:hover:bg-blue-800",
                        onclick: move |_| {
                            // Refresh commits
                            let version_control = props.version_control.clone();
                            spawn(async move {
                                loading.set(true);
                                if let Some(ref history_viewer) = version_control.read().history_viewer {
                                    match history_viewer.get_commit_history(props.branch_filter.as_deref(), props.limit) {
                                        Ok(commit_history) => commits.set(commit_history),
                                        Err(e) => tracing::error!("Failed to refresh commit history: {}", e),
                                    }
                                }
                                loading.set(false);
                            });
                        },
                        "🔄"
                    }
                }
            }
            
            // Commit list
            if *loading.read() {
                div { class: "text-center py-8",
                    div { class: "text-sm text-gray-500 dark:text-gray-400",
                        "Loading history..."
                    }
                }
            } else if commits.read().is_empty() {
                div { class: "text-center py-8 text-gray-500 dark:text-gray-400 text-sm",
                    "No commits found"
                }
            } else {
                div { class: "space-y-2",
                    for commit in commits.read().iter() {
                        CommitItem {
                            commit: commit.clone(),
                            is_selected: selected_commit.read().as_ref() == Some(&commit.id),
                            on_select: move |commit_id: String| {
                                selected_commit.set(Some(commit_id.clone()));
                                show_commit_details.set(true);
                            },
                            on_rollback: props.on_rollback
                        }
                    }
                }
            }
            
            // Commit details modal
            if *show_commit_details.read() {
                if let Some(ref commit_id) = *selected_commit.read() {
                    CommitDetailsModal {
                        commit_id: commit_id.clone(),
                        version_control: props.version_control,
                        on_close: move |_| {
                            show_commit_details.set(false);
                            selected_commit.set(None);
                        },
                        on_rollback: props.on_rollback
                    }
                }
            }
        }
    }
}

#[component]
fn CommitItem(
    commit: CommitInfo,
    is_selected: bool,
    on_select: EventHandler<String>,
    on_rollback: EventHandler<String>
) -> Element {
    let formatted_time = format_commit_time(&commit.timestamp);
    
    rsx! {
        div { 
            class: format!(
                "p-3 border border-gray-200 dark:border-gray-700 rounded-md cursor-pointer transition-colors {}",
                if is_selected {
                    "bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800"
                } else {
                    "hover:bg-gray-50 dark:hover:bg-gray-800"
                }
            ),
            onclick: move |_| on_select.call(commit.id.clone()),
            
            div { class: "flex items-start justify-between",
                div { class: "min-w-0 flex-1",
                    // Commit message
                    div { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-1",
                        "{get_commit_title(&commit.message)}"
                    }
                    
                    // Commit details
                    div { class: "flex items-center space-x-4 text-xs text-gray-500 dark:text-gray-400",
                        div { class: "flex items-center space-x-1",
                            span { "📝" }
                            span { "{commit.short_id}" }
                        }
                        div { class: "flex items-center space-x-1",
                            span { "👤" }
                            span { "{commit.author_name}" }
                        }
                        div { class: "flex items-center space-x-1",
                            span { "⏰" }
                            span { "{formatted_time}" }
                        }
                        if commit.is_merge {
                            span { class: "bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 px-1.5 py-0.5 rounded text-xs",
                                "Merge"
                            }
                        }
                    }
                    
                    // File changes summary
                    if !commit.files_changed.is_empty() {
                        div { class: "mt-2 text-xs text-gray-600 dark:text-gray-400",
                            "{commit.files_changed.len()} files changed"
                            if commit.insertions > 0 || commit.deletions > 0 {
                                span { class: "ml-2",
                                    if commit.insertions > 0 {
                                        span { class: "text-green-600 dark:text-green-400",
                                            "+{commit.insertions} "
                                        }
                                    }
                                    if commit.deletions > 0 {
                                        span { class: "text-red-600 dark:text-red-400",
                                            "-{commit.deletions}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Actions
                div { class: "flex items-center space-x-2 ml-4",
                    button {
                        class: "text-xs px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded hover:bg-gray-200 dark:hover:bg-gray-700",
                        onclick: move |e| {
                            e.stop_propagation();
                            on_select.call(commit.id.clone());
                        },
                        "📋 Details"
                    }
                    Dropdown {
                        trigger: rsx! {
                            button {
                                class: "text-xs px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded hover:bg-blue-200 dark:hover:bg-blue-800",
                                "⚙️"
                            }
                        },
                        items: vec![
                            DropdownItem {
                                label: "Rollback to this commit".to_string(),
                                icon: Some("⏪".to_string()),
                                onclick: Some(Box::new(move || {
                                    on_rollback.call(commit.id.clone());
                                }))
                            }
                        ]
                    }
                }
            }
        }
    }
}

#[component]
fn BranchFilter(
    version_control: Signal<crate::VersionControlState>,
    current_filter: Option<String>
) -> Element {
    let mut branches = use_signal(|| Vec::<String>::new());
    
    // Load branches
    use_effect(move || {
        let version_control = version_control.clone();
        spawn(async move {
            if let Some(ref git_ops) = version_control.read().git_operations {
                match git_ops.list_branches() {
                    Ok(branch_list) => branches.set(branch_list),
                    Err(e) => tracing::error!("Failed to load branches: {}", e),
                }
            }
        });
    });

    rsx! {
        Dropdown {
            trigger: rsx! {
                button {
                    class: "text-xs px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-700",
                    "{current_filter.unwrap_or_else(|| \"All branches\".to_string())} ▼"
                }
            },
            items: {
                let mut items = vec![
                    DropdownItem {
                        label: "All branches".to_string(),
                        icon: Some("🌿".to_string()),
                        onclick: Some(Box::new(move || {
                            // Reset filter
                        }))
                    }
                ];
                
                for branch in branches.read().iter() {
                    let branch_name = branch.clone();
                    items.push(DropdownItem {
                        label: branch_name.clone(),
                        icon: Some("📝".to_string()),
                        onclick: Some(Box::new(move || {
                            // Set branch filter
                        }))
                    });
                }
                
                items
            }
        }
    }
}

#[component]
fn CommitDetailsModal(
    commit_id: String,
    version_control: Signal<crate::VersionControlState>,
    on_close: EventHandler<MouseEvent>,
    on_rollback: EventHandler<String>
) -> Element {
    let mut commit_details = use_signal(|| None::<CommitInfo>);
    let mut loading = use_signal(|| true);

    // Load commit details
    use_effect(move || {
        let version_control = version_control.clone();
        let commit_id = commit_id.clone();
        spawn(async move {
            if let Some(ref history_viewer) = version_control.read().history_viewer {
                match history_viewer.get_commit_details(&commit_id) {
                    Ok(details) => commit_details.set(Some(details)),
                    Err(e) => tracing::error!("Failed to load commit details: {}", e),
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        Modal {
            title: "Commit Details",
            visible: true,
            size: "large",
            on_close: on_close,
            
            div { class: "space-y-4",
                if *loading.read() {
                    div { class: "text-center py-8",
                        "Loading commit details..."
                    }
                } else if let Some(ref details) = *commit_details.read() {
                    CommitDetailsContent { 
                        commit: details.clone(),
                        on_rollback: on_rollback
                    }
                } else {
                    div { class: "text-center py-8 text-red-500",
                        "Failed to load commit details"
                    }
                }
            }
        }
    }
}

#[component]
fn CommitDetailsContent(commit: CommitInfo, on_rollback: EventHandler<String>) -> Element {
    rsx! {
        div { class: "space-y-6",
            // Commit info
            div { class: "bg-gray-50 dark:bg-gray-800 p-4 rounded-md",
                div { class: "grid grid-cols-2 gap-4 text-sm",
                    div {
                        span { class: "font-medium text-gray-700 dark:text-gray-300", "Commit ID:" }
                        span { class: "ml-2 font-mono text-gray-900 dark:text-gray-100", "{commit.id}" }
                    }
                    div {
                        span { class: "font-medium text-gray-700 dark:text-gray-300", "Author:" }
                        span { class: "ml-2 text-gray-900 dark:text-gray-100", "{commit.author_name}" }
                    }
                    div {
                        span { class: "font-medium text-gray-700 dark:text-gray-300", "Date:" }
                        span { class: "ml-2 text-gray-900 dark:text-gray-100", "{format_full_time(&commit.timestamp)}" }
                    }
                    div {
                        span { class: "font-medium text-gray-700 dark:text-gray-300", "Parents:" }
                        span { class: "ml-2 font-mono text-gray-900 dark:text-gray-100", 
                            "{commit.parents.len()} parent(s)"
                        }
                    }
                }
            }
            
            // Commit message
            div {
                h4 { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2",
                    "Message"
                }
                div { class: "bg-white dark:bg-gray-900 p-3 border border-gray-200 dark:border-gray-700 rounded-md",
                    pre { class: "text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap",
                        "{commit.message}"
                    }
                }
            }
            
            // Changed files
            if !commit.files_changed.is_empty() {
                div {
                    h4 { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2",
                        "Changed Files ({commit.files_changed.len()})"
                    }
                    div { class: "bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-md divide-y divide-gray-200 dark:divide-gray-700",
                        for file in commit.files_changed.iter() {
                            div { class: "px-3 py-2 text-sm font-mono text-gray-900 dark:text-gray-100",
                                "{file}"
                            }
                        }
                    }
                }
            }
            
            // Actions
            div { class: "flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700",
                button {
                    class: "px-4 py-2 bg-red-600 text-white rounded-md text-sm font-medium hover:bg-red-700",
                    onclick: move |_| on_rollback.call(commit.id.clone()),
                    "⏪ Rollback to this commit"
                }
            }
        }
    }
}

fn get_commit_title(message: &str) -> String {
    message.lines().next().unwrap_or(message).to_string()
}

fn format_commit_time(timestamp: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*timestamp);
    
    if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d ago", duration.num_days())
    } else {
        timestamp.format("%Y-%m-%d").to_string()
    }
}

fn format_full_time(timestamp: &DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}