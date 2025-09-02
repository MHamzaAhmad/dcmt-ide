use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable, GlobalSignal, Owner};
use dioxus_hooks::{use_signal, use_effect};
use super::git_transport::{WebGitClient, GitStatusResponse};
use latex_ide_ui::{Dropdown, DropdownItem, DropdownSize, DropdownVariant};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures;

// Helper function to spawn async tasks
#[cfg(target_arch = "wasm32")]
fn spawn_local<F>(future: F) 
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_local<F>(_future: F) 
where
    F: std::future::Future<Output = ()> + 'static,
{
    // No-op for non-WASM platforms
}

#[component]
pub fn WebGitPanel(
    git_client: Signal<WebGitClient>,
    mut git_status: Signal<Option<GitStatusResponse>>,
    #[props(default)] pdf_refresh_trigger: Option<Signal<u32>>,
) -> Element {
    let mut commit_message = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
    let mut available_versions = use_signal(|| Vec::<String>::new());
    let mut selected_version = use_signal(|| None::<String>);
    let mut current_version = use_signal(|| None::<String>);
    
    // Refresh status when panel opens
    use_effect(move || {
        let git_client = git_client.clone();
        let mut git_status = git_status.clone();
        let mut is_loading = is_loading.clone();
        
        spawn_local(async move {
            is_loading.set(true);
            match git_client.read().get_status().await {
                Ok(status) => {
                    git_status.set(Some(status));
                }
                Err(e) => {
                    tracing::error!("Failed to get Git status: {}", e);
                }
            }
            is_loading.set(false);
        });
    });
    
    rsx! {
        div {
            class: "h-full bg-zinc-50 dark:bg-zinc-900 flex flex-col",
            
            // Content
            div { class: "flex-1 overflow-y-auto p-3 space-y-4",
                if *is_loading.read() {
                    div { class: "flex items-center justify-center py-12",
                        div { class: "text-sm text-zinc-500 dark:text-zinc-400",
                            "Loading..."
                        }
                    }
                } else if let Some(ref status) = *git_status.read() {
                    // Repository Status
                    div { class: "space-y-3",
                        // Current Branch
                        div { class: "space-y-1",
                            div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                                "Current Branch"
                            }
                            div { class: "text-sm font-mono text-zinc-900 dark:text-zinc-100 bg-white dark:bg-zinc-800 px-2 py-1 rounded border",
                                {status.current_branch.clone()}
                            }
                        }
                        
                        // Session Branch
                        if let Some(ref session) = status.session_branch {
                            div { class: "space-y-1",
                                div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                                    "Session Branch"
                                }
                                div { class: "text-sm font-mono text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 px-2 py-1 rounded border",
                                    {session.clone()}
                                }
                            }
                        }
                        
                        // Status indicator
                        div { class: "flex items-center space-x-2",
                            div { 
                                class: if status.has_changes { 
                                    "w-2 h-2 bg-orange-500 rounded-full" 
                                } else { 
                                    "w-2 h-2 bg-green-500 rounded-full" 
                                }
                            }
                            span { 
                                class: "text-sm text-zinc-600 dark:text-zinc-400",
                                if status.has_changes { "Working tree dirty" } else { "Working tree clean" }
                            }
                        }
                    }
                    
                    // File Changes
                    if !status.modified_files.is_empty() || !status.staged_files.is_empty() || !status.untracked_files.is_empty() {
                        div { class: "space-y-3",
                            div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                                "Changes"
                            }
                            
                            // Staged files
                            if !status.staged_files.is_empty() {
                                div { class: "space-y-1",
                                    div { class: "text-xs font-medium text-green-600 dark:text-green-400",
                                        "Staged • {status.staged_files.len()}"
                                    }
                                    div { class: "space-y-0.5",
                                        for file in &status.staged_files {
                                            div { class: "text-xs font-mono text-zinc-700 dark:text-zinc-300 px-2 py-1 bg-green-50 dark:bg-green-950/20 rounded border-l-2 border-green-500",
                                                {file.clone()}
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Modified files
                            if !status.modified_files.is_empty() {
                                div { class: "space-y-1",
                                    div { class: "text-xs font-medium text-orange-600 dark:text-orange-400",
                                        "Modified • {status.modified_files.len()}"
                                    }
                                    div { class: "space-y-0.5",
                                        for file in &status.modified_files {
                                            div { class: "text-xs font-mono text-zinc-700 dark:text-zinc-300 px-2 py-1 bg-orange-50 dark:bg-orange-950/20 rounded border-l-2 border-orange-500",
                                                {file.clone()}
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Untracked files
                            if !status.untracked_files.is_empty() {
                                div { class: "space-y-1",
                                    div { class: "text-xs font-medium text-zinc-600 dark:text-zinc-400",
                                        "Untracked • {status.untracked_files.len()}"
                                    }
                                    div { class: "space-y-0.5",
                                        for file in &status.untracked_files {
                                            div { class: "text-xs font-mono text-zinc-700 dark:text-zinc-300 px-2 py-1 bg-zinc-50 dark:bg-zinc-800 rounded border-l-2 border-zinc-400",
                                                {file.clone()}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Version Control Section
                    div { class: "space-y-3 border-t border-zinc-200 dark:border-zinc-700 pt-4",
                        div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                            "PDF Versions"
                        }
                        
                        // Current Version Display
                        if let Some(version) = current_version.read().as_ref() {
                            div { class: "text-sm font-mono text-zinc-700 dark:text-zinc-300 bg-blue-50 dark:bg-blue-950/20 px-2 py-1 rounded border-l-2 border-blue-500",
                                "Current: {version}"
                            }
                        } else {
                            div { class: "text-xs text-zinc-500 dark:text-zinc-400",
                                "No versions available"
                            }
                        }
                        
                        // Version Dropdown
                        if !available_versions.read().is_empty() {
                            div { class: "space-y-2",
                                Dropdown {
                                    items: available_versions.read().iter().map(|v| {
                                        DropdownItem::new(v.clone(), v.clone())
                                            .with_icon("🏷️".to_string())
                                            .with_description(format!("Switch to version {}", v))
                                    }).collect(),
                                    selected: selected_version.read().clone(),
                                    onselect: move |version: String| {
                                        selected_version.set(Some(version));
                                    },
                                    placeholder: "Select Version".to_string(),
                                    size: Some(DropdownSize::Small),
                                    variant: Some(DropdownVariant::Default),
                                }
                                
                                // Quick actions
                                div { class: "flex space-x-2",
                                    button {
                                        class: "flex-1 px-2 py-1 bg-orange-600 hover:bg-orange-700 text-white rounded-md text-xs font-medium transition-colors disabled:opacity-50",
                                        disabled: selected_version.read().is_none(),
                                        onclick: move |_| {
                                            if let Some(version) = selected_version.read().as_ref() {
                                                tracing::info!("Rollback to version: {}", version);
                                                // TODO: Implement rollback functionality via git-manager
                                                
                                                // Trigger PDF refresh after rollback
                                                if let Some(mut trigger) = pdf_refresh_trigger {
                                                    let current = *trigger.read();
                                                    trigger.set(current + 1);
                                                    tracing::info!("Triggered PDF refresh after rollback");
                                                }
                                            }
                                        },
                                        "Rollback"
                                    }
                                    
                                    button {
                                        class: "px-2 py-1 bg-red-600 hover:bg-red-700 text-white rounded-md text-xs font-medium transition-colors",
                                        onclick: move |_| {
                                            // TODO: Quick rollback to previous commit
                                            tracing::info!("Quick rollback to previous commit");
                                            
                                            // Trigger PDF refresh after quick rollback
                                            if let Some(mut trigger) = pdf_refresh_trigger {
                                                let current = *trigger.read();
                                                trigger.set(current + 1);
                                                tracing::info!("Triggered PDF refresh after quick rollback");
                                            }
                                        },
                                        "⏪ Quick"
                                    }
                                }
                            }
                        }
                    }
                    
                    // Actions
                    div { class: "space-y-3",
                        div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                            "Actions"
                        }
                        
                        div { class: "space-y-2",
                            if status.session_branch.is_none() {
                                button {
                                    class: "w-full px-3 py-2 bg-zinc-900 hover:bg-zinc-800 dark:bg-zinc-100 dark:hover:bg-zinc-200 text-white dark:text-zinc-900 rounded-md text-sm font-medium transition-colors",
                                    onclick: move |_| {
                                        let git_client = git_client.clone();
                                        let mut git_status = git_status.clone();
                                        
                                        spawn_local(async move {
                                            match git_client.read().start_session().await {
                                                Ok(session_branch) => {
                                                    tracing::info!("Started Git session: {}", session_branch);
                                                    // Refresh status
                                                    if let Ok(status) = git_client.read().get_status().await {
                                                        git_status.set(Some(status));
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to start Git session: {}", e);
                                                }
                                            }
                                        });
                                    },
                                    "Start Session"
                                }
                            }
                            
                            button {
                                class: "w-full px-3 py-2 bg-white hover:bg-zinc-50 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-zinc-900 dark:text-zinc-100 border border-zinc-200 dark:border-zinc-700 rounded-md text-sm font-medium transition-colors",
                                onclick: move |_| {
                                    let git_client = git_client.clone();
                                    let mut git_status = git_status.clone();
                                    let mut is_loading = is_loading.clone();
                                    
                                    spawn_local(async move {
                                        is_loading.set(true);
                                        match git_client.read().get_status().await {
                                            Ok(status) => {
                                                git_status.set(Some(status));
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to refresh Git status: {}", e);
                                            }
                                        }
                                        is_loading.set(false);
                                    });
                                },
                                "Refresh"
                            }
                        }
                    }
                    
                    // Commit section
                    if status.has_changes {
                        div { class: "space-y-3 border-t border-zinc-200 dark:border-zinc-700 pt-4",
                            div { class: "text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wide",
                                "Commit"
                            }
                            
                            textarea {
                                class: "w-full h-20 px-3 py-2 text-sm border border-zinc-200 dark:border-zinc-700 rounded-md bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 resize-none focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 placeholder:text-zinc-400 dark:placeholder:text-zinc-500",
                                placeholder: "Commit message",
                                value: "{commit_message}",
                                oninput: move |evt| {
                                    commit_message.set(evt.value());
                                }
                            }
                            
                            button {
                                class: if commit_message.read().is_empty() {
                                    "w-full px-3 py-2 bg-zinc-200 dark:bg-zinc-700 text-zinc-400 dark:text-zinc-500 rounded-md text-sm font-medium cursor-not-allowed"
                                } else {
                                    "w-full px-3 py-2 bg-zinc-900 hover:bg-zinc-800 dark:bg-zinc-100 dark:hover:bg-zinc-200 text-white dark:text-zinc-900 rounded-md text-sm font-medium transition-colors"
                                },
                                disabled: commit_message.read().is_empty(),
                                onclick: move |_| {
                                    let message = commit_message.read().clone();
                                    if !message.is_empty() {
                                        let git_client = git_client.clone();
                                        let mut git_status = git_status.clone();
                                        let mut commit_message = commit_message.clone();
                                        
                                        spawn_local(async move {
                                            // Stage all changes first
                                            if let Err(e) = git_client.read().stage_all_changes().await {
                                                tracing::error!("Failed to stage changes: {}", e);
                                                return;
                                            }
                                            
                                            // Commit
                                            match git_client.read().commit(message).await {
                                                Ok(commit_info) => {
                                                    tracing::info!("Created commit: {}", commit_info.short_id);
                                                    commit_message.set(String::new());
                                                    
                                                    // Refresh status
                                                    if let Ok(status) = git_client.read().get_status().await {
                                                        git_status.set(Some(status));
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to commit: {}", e);
                                                }
                                            }
                                        });
                                    }
                                },
                                "Commit Changes"
                            }
                        }
                    }
                } else {
                    div { class: "flex items-center justify-center py-12",
                        div { class: "text-sm text-zinc-500 dark:text-zinc-400",
                            "No repository"
                        }
                    }
                }
            }
        }
    }
}