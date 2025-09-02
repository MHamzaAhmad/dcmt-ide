use dioxus::prelude::*;
use latex_ide_ui::*;

#[derive(Props, Clone, PartialEq)]
pub struct BranchSelectorProps {
    pub version_control: Signal<crate::VersionControlState>,
}

#[component]
pub fn BranchSelector(props: BranchSelectorProps) -> Element {
    let mut branches = use_signal(|| Vec::<String>::new());
    let mut current_branch = use_signal(|| String::new());
    let mut new_branch_name = use_signal(|| String::new());
    let mut show_create_branch = use_signal(|| false);
    let mut loading = use_signal(|| false);

    // Load branches
    use_effect(move || {
        let version_control = props.version_control.clone();
        spawn(async move {
            if let Some(ref git_ops) = version_control.read().git_operations {
                match git_ops.list_branches() {
                    Ok(branch_list) => branches.set(branch_list),
                    Err(e) => tracing::error!("Failed to load branches: {}", e),
                }
                
                match git_ops.get_current_branch() {
                    Ok(branch) => current_branch.set(branch),
                    Err(e) => tracing::error!("Failed to get current branch: {}", e),
                }
            }
        });
    });

    rsx! {
        div { class: "space-y-4",
            
            // Current branch info
            div { class: "p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md",
                div { class: "text-sm font-medium text-blue-900 dark:text-blue-100",
                    "Current Branch"
                }
                div { class: "text-lg font-mono text-blue-800 dark:text-blue-200 mt-1",
                    "🌿 {current_branch}"
                }
                if let Some(session_branch) = props.version_control.read().get_session_branch() {
                    div { class: "text-xs text-blue-600 dark:text-blue-300 mt-1",
                        "Session: {session_branch}"
                    }
                }
            }
            
            // Create new branch
            div { class: "space-y-2",
                button {
                    class: "w-full text-left px-3 py-2 border border-dashed border-gray-300 dark:border-gray-600 rounded-md text-sm text-gray-600 dark:text-gray-400 hover:border-gray-400 dark:hover:border-gray-500 hover:text-gray-800 dark:hover:text-gray-300",
                    onclick: move |_| show_create_branch.set(!*show_create_branch.read()),
                    "➕ Create New Branch"
                }
                
                if *show_create_branch.read() {
                    div { class: "p-3 border border-gray-200 dark:border-gray-700 rounded-md space-y-3",
                        Input {
                            placeholder: "Branch name (e.g., feature/new-chapter)",
                            value: new_branch_name.read().clone(),
                            oninput: move |value| new_branch_name.set(value)
                        }
                        
                        div { class: "flex space-x-2",
                            button {
                                class: "flex-1 bg-green-600 text-white px-3 py-2 rounded-md text-sm font-medium hover:bg-green-700 disabled:opacity-50",
                                disabled: new_branch_name.read().trim().is_empty() || *loading.read(),
                                onclick: move |_| {
                                    let branch_name = new_branch_name.read().clone();
                                    let version_control = props.version_control.clone();
                                    spawn(async move {
                                        loading.set(true);
                                        if let Some(ref git_ops) = version_control.read().git_operations {
                                            match git_ops.create_branch(&branch_name, true) {
                                                Ok(_) => {
                                                    new_branch_name.set(String::new());
                                                    show_create_branch.set(false);
                                                    // Refresh branches list
                                                    if let Ok(branch_list) = git_ops.list_branches() {
                                                        branches.set(branch_list);
                                                    }
                                                }
                                                Err(e) => tracing::error!("Failed to create branch: {}", e),
                                            }
                                        }
                                        loading.set(false);
                                    });
                                },
                                "Create"
                            }
                            button {
                                class: "flex-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-3 py-2 rounded-md text-sm font-medium hover:bg-gray-200 dark:hover:bg-gray-700",
                                onclick: move |_| {
                                    show_create_branch.set(false);
                                    new_branch_name.set(String::new());
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
            
            // Branch list
            div { class: "space-y-2",
                h4 { class: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2",
                    "All Branches ({branches.read().len()})"
                }
                
                if branches.read().is_empty() {
                    div { class: "text-center py-4 text-gray-500 dark:text-gray-400 text-sm",
                        "No branches found"
                    }
                } else {
                    div { class: "space-y-1",
                        for branch in branches.read().iter() {
                            BranchItem {
                                name: branch.clone(),
                                is_current: branch == &*current_branch.read(),
                                version_control: props.version_control,
                                on_branch_changed: move |new_branch: String| {
                                    current_branch.set(new_branch);
                                    // Refresh branches list
                                    let version_control = props.version_control.clone();
                                    spawn(async move {
                                        if let Some(ref git_ops) = version_control.read().git_operations {
                                            if let Ok(branch_list) = git_ops.list_branches() {
                                                branches.set(branch_list);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BranchItem(
    name: String,
    is_current: bool,
    version_control: Signal<crate::VersionControlState>,
    on_branch_changed: EventHandler<String>
) -> Element {
    let mut show_menu = use_signal(|| false);

    rsx! {
        div { 
            class: format!(
                "flex items-center justify-between p-2 rounded-md {}",
                if is_current {
                    "bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800"
                } else {
                    "hover:bg-gray-50 dark:hover:bg-gray-800"
                }
            ),
            
            div { class: "flex items-center space-x-2 min-w-0 flex-1",
                div { class: format!("w-3 h-3 rounded-full flex-shrink-0 {}", if is_current { "bg-blue-500" } else { "bg-gray-300 dark:bg-gray-600" }) }
                
                div { class: "min-w-0 flex-1",
                    div { class: format!("text-sm font-mono truncate {}", if is_current { "text-blue-900 dark:text-blue-100 font-medium" } else { "text-gray-900 dark:text-gray-100" }),
                        "{name}"
                    }
                    if is_current {
                        div { class: "text-xs text-blue-600 dark:text-blue-300",
                            "Current branch"
                        }
                    }
                }
            }
            
            div { class: "flex items-center space-x-1",
                if !is_current {
                    button {
                        class: "text-xs px-2 py-1 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded hover:bg-blue-200 dark:hover:bg-blue-800",
                        onclick: move |_| {
                            let branch_name = name.clone();
                            let version_control = version_control.clone();
                            spawn(async move {
                                if let Some(ref git_ops) = version_control.read().git_operations {
                                    match git_ops.checkout_branch(&branch_name) {
                                        Ok(_) => {
                                            on_branch_changed.call(branch_name.clone());
                                        }
                                        Err(e) => tracing::error!("Failed to checkout branch: {}", e),
                                    }
                                }
                            });
                        },
                        "Switch"
                    }
                }
                
                Dropdown {
                    trigger: rsx! {
                        button {
                            class: "text-xs p-1 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300",
                            "⋮"
                        }
                    },
                    items: {
                        let mut items = Vec::new();
                        
                        if !is_current {
                            items.push(DropdownItem {
                                label: "Merge into current".to_string(),
                                icon: Some("🔀".to_string()),
                                onclick: Some(Box::new(move || {
                                    let branch_name = name.clone();
                                    let version_control = version_control.clone();
                                    spawn(async move {
                                        if let Some(ref git_ops) = version_control.read().git_operations {
                                            match git_ops.merge_branch(&branch_name, None) {
                                                Ok(_) => {
                                                    tracing::info!("Merged branch: {}", branch_name);
                                                }
                                                Err(e) => tracing::error!("Failed to merge branch: {}", e),
                                            }
                                        }
                                    });
                                }))
                            });
                            
                            items.push(DropdownItem {
                                label: "Delete branch".to_string(),
                                icon: Some("🗑️".to_string()),
                                onclick: Some(Box::new(move || {
                                    let branch_name = name.clone();
                                    let version_control = version_control.clone();
                                    spawn(async move {
                                        if let Some(ref git_ops) = version_control.read().git_operations {
                                            match git_ops.delete_branch(&branch_name, false) {
                                                Ok(_) => {
                                                    tracing::info!("Deleted branch: {}", branch_name);
                                                }
                                                Err(e) => tracing::error!("Failed to delete branch: {}", e),
                                            }
                                        }
                                    });
                                }))
                            });
                        }
                        
                        items
                    }
                }
            }
        }
    }
}