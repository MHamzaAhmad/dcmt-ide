use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_git_manager::{GitStatus, CommitInfo};
use crate::{use_version_control, HistoryView, BranchSelector, FileChangesList, ConflictResolverUI};

#[derive(Props, Clone, PartialEq)]
pub struct GitPanelProps {
    #[props(default = true)]
    pub visible: bool,
    #[props(default = 350)]
    pub width: u32,
    #[props(default)]
    pub on_close: Option<EventHandler<()>>,
}

#[component]
pub fn GitPanel(props: GitPanelProps) -> Element {
    let mut version_control = use_version_control();
    let mut active_tab = use_signal(|| "changes".to_string());
    let mut commit_message = use_signal(|| String::new());
    let mut tag_name = use_signal(|| String::new());
    let mut show_rollback_confirm = use_signal(|| false);
    let mut selected_commit_for_rollback = use_signal(|| None::<String>);

    if !props.visible {
        return rsx! { div {} };
    }

    let current_status = version_control.read().current_status.clone();
    let has_conflicts = version_control.read().conflict_resolver.as_ref()
        .and_then(|resolver| resolver.has_conflicts().ok())
        .unwrap_or(false);

    rsx! {
        div { 
            class: "git-panel fixed right-0 top-0 h-full bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700 shadow-lg z-50",
            style: "width: {props.width}px",
            
            // Header
            div { class: "p-4 border-b border-gray-200 dark:border-gray-700",
                div { class: "flex items-center justify-between mb-4",
                    h3 { class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                        "Git Version Control"
                    }
                    if let Some(on_close) = props.on_close {
                        button {
                            class: "p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
                    }
                }
                
                // Repository status
                if let Some(status) = &current_status {
                    RepositoryStatus { status: status.clone() }
                } else {
                    div { class: "text-sm text-gray-500 dark:text-gray-400",
                        "No Git repository found"
                    }
                }
            }
            
            // Tab navigation
            div { class: "border-b border-gray-200 dark:border-gray-700",
                div { class: "flex",
                    TabButton {
                        label: "Changes",
                        active: *active_tab.read() == "changes",
                        onclick: move |_| active_tab.set("changes".to_string()),
                        has_indicator: current_status.as_ref().map_or(false, |s| s.has_changes)
                    }
                    TabButton {
                        label: "History",
                        active: *active_tab.read() == "history",
                        onclick: move |_| active_tab.set("history".to_string())
                    }
                    TabButton {
                        label: "Branches",
                        active: *active_tab.read() == "branches",
                        onclick: move |_| active_tab.set("branches".to_string())
                    }
                    if has_conflicts {
                        TabButton {
                            label: "Conflicts",
                            active: *active_tab.read() == "conflicts",
                            onclick: move |_| active_tab.set("conflicts".to_string()),
                            has_indicator: true
                        }
                    }
                }
            }
            
            // Tab content
            div { class: "flex-1 overflow-y-auto",
                match active_tab.read().as_str() {
                    "changes" => rsx! {
                        ChangesTab {
                            status: current_status.clone(),
                            commit_message: commit_message,
                            tag_name: tag_name,
                            version_control: version_control
                        }
                    },
                    "history" => rsx! {
                        HistoryTab {
                            version_control: version_control,
                            show_rollback_confirm: show_rollback_confirm,
                            selected_commit_for_rollback: selected_commit_for_rollback
                        }
                    },
                    "branches" => rsx! {
                        BranchesTab { version_control: version_control }
                    },
                    "conflicts" => rsx! {
                        ConflictsTab { version_control: version_control }
                    },
                    _ => rsx! { div {} }
                }
            }
            
            // Rollback confirmation modal
            if *show_rollback_confirm.read() {
                RollbackConfirmModal {
                    commit_id: selected_commit_for_rollback.read().clone().unwrap_or_default(),
                    version_control: version_control,
                    on_confirm: move |commit_id: String| {
                        spawn(async move {
                            // Perform rollback
                            if let Some(ref conflict_resolver) = version_control.read().conflict_resolver {
                                match conflict_resolver.safe_rollback_to_commit(&commit_id) {
                                    Ok(_) => {
                                        let _ = version_control.write().refresh_status();
                                    },
                                    Err(e) => {
                                        tracing::error!("Rollback failed: {}", e);
                                    }
                                }
                            }
                        });
                        show_rollback_confirm.set(false);
                        selected_commit_for_rollback.set(None);
                    },
                    on_cancel: move || {
                        show_rollback_confirm.set(false);
                        selected_commit_for_rollback.set(None);
                    }
                }
            }
        }
    }
}

#[component]
fn RepositoryStatus(status: GitStatus) -> Element {
    rsx! {
        div { class: "space-y-2",
            div { class: "flex items-center justify-between",
                div { class: "text-sm font-medium text-gray-700 dark:text-gray-300",
                    "Current: {status.current_branch}"
                }
                div { class: "text-xs text-gray-500 dark:text-gray-400",
                    if let Some(ref session) = status.session_branch {
                        "Session: {session}"
                    } else {
                        "No active session"
                    }
                }
            }
            
            if status.has_changes {
                div { class: "text-xs text-orange-600 dark:text-orange-400",
                    "● {status.staged_files.len() + status.modified_files.len() + status.untracked_files.len()} changes"
                }
            } else {
                div { class: "text-xs text-green-600 dark:text-green-400",
                    "✓ No changes"
                }
            }
        }
    }
}

#[component]
fn TabButton(
    label: String,
    active: bool,
    has_indicator: Option<bool>,
    onclick: EventHandler<MouseEvent>
) -> Element {
    let indicator = has_indicator.unwrap_or(false);
    
    rsx! {
        button {
            class: format!(
                "px-4 py-2 text-sm font-medium border-b-2 relative {}",
                if active {
                    "text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-400"
                } else {
                    "text-gray-500 dark:text-gray-400 border-transparent hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600"
                }
            ),
            onclick: onclick,
            {label}
            if indicator {
                span { class: "absolute top-1 right-1 w-2 h-2 bg-orange-500 rounded-full" }
            }
        }
    }
}

#[component]
fn ChangesTab(
    status: Option<GitStatus>,
    mut commit_message: Signal<String>,
    mut tag_name: Signal<String>,
    mut version_control: Signal<crate::VersionControlState>
) -> Element {
    let has_changes = status.as_ref().map_or(false, |s| s.has_changes);
    
    rsx! {
        div { class: "p-4",
            if let Some(ref status) = status {
                FileChangesList { status: status.clone() }
                
                if has_changes {
                    div { class: "mt-4 space-y-4",
                        // Commit message input
                        div {
                            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                                "Commit message"
                            }
                            textarea {
                                class: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm resize-none bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100",
                                placeholder: "Describe your changes...",
                                rows: "3",
                                value: "{commit_message}",
                                oninput: move |evt| commit_message.set(evt.value())
                            }
                        }
                        
                        // Commit button
                        button {
                            class: "w-full bg-blue-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: commit_message.read().trim().is_empty(),
                            onclick: move |_| {
                                let message = commit_message.read().clone();
                                spawn(async move {
                                    if let Some(ref session_manager) = version_control.read().session_manager {
                                        if let Ok(_) = session_manager.commit_session_changes(&message) {
                                            commit_message.set(String::new());
                                            let _ = version_control.write().refresh_status();
                                        }
                                    }
                                });
                            },
                            "💾 Commit Changes"
                        }
                        
                        // Save session with tag
                        div { class: "border-t border-gray-200 dark:border-gray-700 pt-4",
                            h4 { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2",
                                "Save Session"
                            }
                            Input {
                                placeholder: "Tag name (e.g., v1.0, milestone-1)",
                                value: tag_name.read().clone(),
                                oninput: move |evt| tag_name.set(evt)
                            }
                            button {
                                class: "mt-2 w-full bg-green-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-green-700 disabled:opacity-50",
                                disabled: tag_name.read().trim().is_empty() || commit_message.read().trim().is_empty(),
                                onclick: move |_| {
                                    let tag = tag_name.read().clone();
                                    let message = commit_message.read().clone();
                                    spawn(async move {
                                        if let Some(ref session_manager) = version_control.read().session_manager {
                                            if let Ok(_) = session_manager.save_session_with_tag(&tag, &message) {
                                                tag_name.set(String::new());
                                                commit_message.set(String::new());
                                                let _ = version_control.write().refresh_status();
                                            }
                                        }
                                    });
                                },
                                "🏷️ Save & Tag"
                            }
                        }
                    }
                } else {
                    div { class: "text-center py-8 text-gray-500 dark:text-gray-400",
                        "No changes to commit"
                    }
                }
            } else {
                div { class: "text-center py-8 text-gray-500 dark:text-gray-400",
                    "Repository not initialized"
                }
            }
        }
    }
}

#[component]
fn HistoryTab(
    mut version_control: Signal<crate::VersionControlState>,
    mut show_rollback_confirm: Signal<bool>,
    mut selected_commit_for_rollback: Signal<Option<String>>
) -> Element {
    rsx! {
        div { class: "p-4",
            HistoryView {
                version_control: version_control,
                on_rollback: move |commit_id: String| {
                    selected_commit_for_rollback.set(Some(commit_id));
                    show_rollback_confirm.set(true);
                }
            }
        }
    }
}

#[component]
fn BranchesTab(version_control: Signal<crate::VersionControlState>) -> Element {
    rsx! {
        div { class: "p-4",
            BranchSelector { version_control: version_control }
        }
    }
}

#[component]
fn ConflictsTab(version_control: Signal<crate::VersionControlState>) -> Element {
    rsx! {
        div { class: "p-4",
            ConflictResolverUI { version_control: version_control }
        }
    }
}

#[component]
fn RollbackConfirmModal(
    commit_id: String,
    version_control: Signal<crate::VersionControlState>,
    on_confirm: EventHandler<String>,
    on_cancel: EventHandler<()>
) -> Element {
    let has_changes = version_control.read().has_changes();
    
    rsx! {
        Modal {
            title: "Confirm Rollback",
            visible: true,
            on_close: move |_| on_cancel.call(()),
            
            div { class: "space-y-4",
                div { class: "text-sm text-gray-700 dark:text-gray-300",
                    "Are you sure you want to rollback to commit "
                    code { class: "bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs",
                        "{commit_id[..7.min(commit_id.len())]}"
                    }
                    "?"
                }
                
                if has_changes {
                    div { class: "p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-md",
                        div { class: "text-sm text-orange-800 dark:text-orange-200 font-medium",
                            "⚠️ You have uncommitted changes"
                        }
                        div { class: "text-xs text-orange-600 dark:text-orange-300 mt-1",
                            "These changes will be lost during rollback. Consider committing them first."
                        }
                    }
                }
                
                div { class: "flex space-x-3 pt-4",
                    button {
                        class: "flex-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-4 py-2 rounded-md text-sm font-medium hover:bg-gray-200 dark:hover:bg-gray-700",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "flex-1 bg-red-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-red-700",
                        onclick: move |_| on_confirm.call(commit_id.clone()),
                        "Rollback"
                    }
                }
            }
        }
    }
}