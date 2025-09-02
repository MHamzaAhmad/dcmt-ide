use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_git_manager::{ConflictInfo, ConflictType, Resolution, ResolutionChoice, CleanupOptions};

#[derive(Props, Clone, PartialEq)]
pub struct ConflictResolverUIProps {
    pub version_control: Signal<crate::VersionControlState>,
}

#[component]
pub fn ConflictResolverUI(props: ConflictResolverUIProps) -> Element {
    let mut conflicts = use_signal(|| Vec::<ConflictInfo>::new());
    let mut resolutions = use_signal(|| Vec::<ResolutionChoice>::new());
    let mut loading = use_signal(|| false);
    let mut show_cleanup_dialog = use_signal(|| false);

    // Load conflicts
    use_effect(move || {
        let version_control = props.version_control.clone();
        spawn(async move {
            if let Some(ref conflict_resolver) = version_control.read().conflict_resolver {
                match conflict_resolver.get_conflicts() {
                    Ok(conflict_list) => conflicts.set(conflict_list),
                    Err(e) => tracing::error!("Failed to load conflicts: {}", e),
                }
            }
        });
    });

    if conflicts.read().is_empty() {
        return rsx! {
            div { class: "space-y-4",
                div { class: "text-center py-8",
                    div { class: "text-4xl mb-4", "✅" }
                    div { class: "text-lg font-medium text-gray-900 dark:text-gray-100",
                        "No Conflicts"
                    }
                    div { class: "text-sm text-gray-500 dark:text-gray-400",
                        "All changes are clean and ready to commit"
                    }
                }
                
                // Cleanup options
                CleanupSection {
                    version_control: props.version_control,
                    show_cleanup_dialog: show_cleanup_dialog
                }
            }
        };
    }

    rsx! {
        div { class: "space-y-4",
            
            // Header
            div { class: "flex items-center justify-between",
                h4 { class: "text-sm font-medium text-red-900 dark:text-red-100",
                    "⚠️ Merge Conflicts ({conflicts.read().len()})"
                }
                div { class: "flex space-x-2",
                    button {
                        class: "text-xs px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-700",
                        onclick: move |_| show_cleanup_dialog.set(true),
                        "🧹 Cleanup"
                    }
                    button {
                        class: "text-xs px-2 py-1 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300 rounded hover:bg-red-200 dark:hover:bg-red-800",
                        onclick: move |_| {
                            let version_control = props.version_control.clone();
                            spawn(async move {
                                if let Some(ref conflict_resolver) = version_control.read().conflict_resolver {
                                    match conflict_resolver.abort_merge() {
                                        Ok(_) => {
                                            conflicts.set(Vec::new());
                                            tracing::info!("Merge aborted successfully");
                                        }
                                        Err(e) => tracing::error!("Failed to abort merge: {}", e),
                                    }
                                }
                            });
                        },
                        "❌ Abort Merge"
                    }
                }
            }
            
            // Conflicts list
            div { class: "space-y-3",
                for (i, conflict) in conflicts.read().iter().enumerate() {
                    ConflictItem {
                        conflict: conflict.clone(),
                        resolution: resolutions.read().get(i).cloned(),
                        on_resolution_change: move |choice: ResolutionChoice| {
                            let mut current_resolutions = resolutions.read().clone();
                            if i < current_resolutions.len() {
                                current_resolutions[i] = choice;
                            } else {
                                current_resolutions.resize(i + 1, ResolutionChoice {
                                    file_path: String::new(),
                                    resolution: Resolution::Manual,
                                    custom_content: None,
                                });
                                current_resolutions[i] = choice;
                            }
                            resolutions.set(current_resolutions);
                        }
                    }
                }
            }
            
            // Resolve conflicts button
            div { class: "pt-4 border-t border-gray-200 dark:border-gray-700",
                button {
                    class: "w-full bg-green-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-green-700 disabled:opacity-50",
                    disabled: resolutions.read().len() < conflicts.read().len() || *loading.read(),
                    onclick: move |_| {
                        let version_control = props.version_control.clone();
                        let resolution_choices = resolutions.read().clone();
                        spawn(async move {
                            loading.set(true);
                            if let Some(ref conflict_resolver) = version_control.read().conflict_resolver {
                                match conflict_resolver.resolve_conflicts(resolution_choices) {
                                    Ok(_) => {
                                        conflicts.set(Vec::new());
                                        resolutions.set(Vec::new());
                                        let _ = version_control.write().refresh_status();
                                        tracing::info!("Conflicts resolved successfully");
                                    }
                                    Err(e) => tracing::error!("Failed to resolve conflicts: {}", e),
                                }
                            }
                            loading.set(false);
                        });
                    },
                    if *loading.read() { "Resolving..." } else { "✅ Resolve All Conflicts" }
                }
            }
            
            // Cleanup dialog
            if *show_cleanup_dialog.read() {
                CleanupDialog {
                    version_control: props.version_control,
                    on_close: move |_| show_cleanup_dialog.set(false)
                }
            }
        }
    }
}

#[component]
fn ConflictItem(
    conflict: ConflictInfo,
    resolution: Option<ResolutionChoice>,
    on_resolution_change: EventHandler<ResolutionChoice>
) -> Element {
    let mut expanded = use_signal(|| false);
    let mut custom_content = use_signal(|| String::new());
    let mut selected_resolution = use_signal(|| Resolution::Manual);

    rsx! {
        div { class: "border border-red-200 dark:border-red-800 rounded-md bg-red-50 dark:bg-red-900/20",
            
            // Header
            div { 
                class: "px-4 py-3 cursor-pointer",
                onclick: move |_| expanded.set(!*expanded.read()),
                
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center space-x-3",
                        div { class: "text-red-600 dark:text-red-400",
                            "{get_conflict_icon(&conflict.conflict_type)}"
                        }
                        div {
                            div { class: "text-sm font-medium text-red-900 dark:text-red-100",
                                "{conflict.file_path}"
                            }
                            div { class: "text-xs text-red-600 dark:text-red-400",
                                "{get_conflict_description(&conflict.conflict_type)}"
                            }
                        }
                    }
                    div { class: "text-red-500 dark:text-red-400 text-sm",
                        if *expanded.read() { "▼" } else { "▶" }
                    }
                }
            }
            
            // Expanded content
            if *expanded.read() {
                div { class: "border-t border-red-200 dark:border-red-800 p-4 space-y-4",
                    
                    // Resolution options
                    div { class: "space-y-2",
                        h5 { class: "text-sm font-medium text-red-900 dark:text-red-100",
                            "Resolution Options"
                        }
                        
                        ResolutionRadioGroup {
                            conflict: conflict.clone(),
                            selected: selected_resolution,
                            on_change: move |resolution: Resolution| {
                                selected_resolution.set(resolution.clone());
                                
                                let choice = ResolutionChoice {
                                    file_path: conflict.file_path.clone(),
                                    resolution: resolution.clone(),
                                    custom_content: if matches!(resolution, Resolution::Custom) {
                                        Some(custom_content.read().clone())
                                    } else {
                                        None
                                    }
                                };
                                on_resolution_change.call(choice);
                            }
                        }
                    }
                    
                    // Custom content editor (if custom resolution selected)
                    if matches!(*selected_resolution.read(), Resolution::Custom) {
                        div { class: "space-y-2",
                            label { class: "text-sm font-medium text-red-900 dark:text-red-100",
                                "Custom Content"
                            }
                            textarea {
                                class: "w-full h-32 p-3 border border-red-300 dark:border-red-600 rounded-md text-sm font-mono bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",
                                placeholder: "Enter the resolved content for this file...",
                                value: "{custom_content}",
                                oninput: move |evt| {
                                    custom_content.set(evt.value());
                                    let choice = ResolutionChoice {
                                        file_path: conflict.file_path.clone(),
                                        resolution: Resolution::Custom,
                                        custom_content: Some(evt.value())
                                    };
                                    on_resolution_change.call(choice);
                                }
                            }
                        }
                    }
                    
                    // Content preview
                    ConflictContentPreview { conflict: conflict.clone() }
                }
            }
        }
    }
}

#[component]
fn ResolutionRadioGroup(
    conflict: ConflictInfo,
    mut selected: Signal<Resolution>,
    on_change: EventHandler<Resolution>
) -> Element {
    rsx! {
        div { class: "space-y-2",
            ResolutionOption {
                resolution: Resolution::TakeOurs,
                label: "Take our version",
                description: "Keep the version from the current branch",
                icon: "👈",
                selected: matches!(*selected.read(), Resolution::TakeOurs),
                enabled: conflict.our_content.is_some(),
                on_select: move |res| {
                    selected.set(res.clone());
                    on_change.call(res);
                }
            }
            
            ResolutionOption {
                resolution: Resolution::TakeTheirs,
                label: "Take their version",
                description: "Keep the version being merged in",
                icon: "👉",
                selected: matches!(*selected.read(), Resolution::TakeTheirs),
                enabled: conflict.their_content.is_some(),
                on_select: move |res| {
                    selected.set(res.clone());
                    on_change.call(res);
                }
            }
            
            if conflict.base_content.is_some() {
                ResolutionOption {
                    resolution: Resolution::TakeBase,
                    label: "Take base version",
                    description: "Keep the common ancestor version",
                    icon: "🔄",
                    selected: matches!(*selected.read(), Resolution::TakeBase),
                    enabled: true,
                    on_select: move |res| {
                        selected.set(res.clone());
                        on_change.call(res);
                    }
                }
            }
            
            ResolutionOption {
                resolution: Resolution::Custom,
                label: "Custom resolution",
                description: "Manually edit the resolved content",
                icon: "✏️",
                selected: matches!(*selected.read(), Resolution::Custom),
                enabled: true,
                on_select: move |res| {
                    selected.set(res.clone());
                    on_change.call(res);
                }
            }
            
            ResolutionOption {
                resolution: Resolution::Manual,
                label: "Already resolved manually",
                description: "File has been manually resolved in working directory",
                icon: "✅",
                selected: matches!(*selected.read(), Resolution::Manual),
                enabled: true,
                on_select: move |res| {
                    selected.set(res.clone());
                    on_change.call(res);
                }
            }
        }
    }
}

#[component]
fn ResolutionOption(
    resolution: Resolution,
    label: String,
    description: String,
    icon: String,
    selected: bool,
    enabled: bool,
    on_select: EventHandler<Resolution>
) -> Element {
    rsx! {
        label { 
            class: format!(
                "flex items-center p-2 rounded-md cursor-pointer {}",
                if enabled {
                    if selected {
                        "bg-blue-100 dark:bg-blue-900 border border-blue-300 dark:border-blue-700"
                    } else {
                        "hover:bg-gray-100 dark:hover:bg-gray-800 border border-transparent"
                    }
                } else {
                    "opacity-50 cursor-not-allowed"
                }
            ),
            
            input {
                r#type: "radio",
                class: "sr-only",
                disabled: !enabled,
                checked: selected,
                onchange: move |_| {
                    if enabled {
                        on_select.call(resolution.clone());
                    }
                }
            }
            
            div { class: "flex items-center space-x-3",
                span { class: "text-lg", "{icon}" }
                div {
                    div { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                        "{label}"
                    }
                    div { class: "text-xs text-gray-500 dark:text-gray-400",
                        "{description}"
                    }
                }
            }
        }
    }
}

#[component]
fn ConflictContentPreview(conflict: ConflictInfo) -> Element {
    rsx! {
        div { class: "space-y-3",
            h5 { class: "text-sm font-medium text-red-900 dark:text-red-100",
                "Content Preview"
            }
            
            div { class: "grid gap-3",
                if let Some(ref our_content) = conflict.our_content {
                    ContentBlock {
                        title: "Our Version",
                        content: our_content.clone(),
                        color: "blue"
                    }
                }
                
                if let Some(ref their_content) = conflict.their_content {
                    ContentBlock {
                        title: "Their Version",
                        content: their_content.clone(),
                        color: "green"
                    }
                }
                
                if let Some(ref merged_content) = conflict.merged_content {
                    ContentBlock {
                        title: "Current (with conflict markers)",
                        content: merged_content.clone(),
                        color: "red"
                    }
                }
            }
        }
    }
}

#[component]
fn ContentBlock(title: String, content: String, color: String) -> Element {
    let max_lines = 10;
    let lines: Vec<&str> = content.lines().collect();
    let truncated = lines.len() > max_lines;
    let displayed_content = if truncated {
        lines[..max_lines].join("\n") + "\n..."
    } else {
        content.clone()
    };

    rsx! {
        div { class: format!("border border-{}-200 dark:border-{}-700 rounded-md", color, color),
            div { class: format!("px-3 py-2 bg-{}-50 dark:bg-{}-900/20 border-b border-{}-200 dark:border-{}-700", color, color, color, color),
                div { class: format!("text-sm font-medium text-{}-900 dark:text-{}-100", color, color),
                    "{title}"
                }
                if truncated {
                    div { class: format!("text-xs text-{}-600 dark:text-{}-400", color, color),
                        "Showing first {max_lines} lines of {lines.len()}"
                    }
                }
            }
            div { class: "p-3",
                pre { class: "text-xs font-mono text-gray-900 dark:text-gray-100 whitespace-pre-wrap overflow-x-auto",
                    "{displayed_content}"
                }
            }
        }
    }
}

#[component]
fn CleanupSection(
    version_control: Signal<crate::VersionControlState>,
    mut show_cleanup_dialog: Signal<bool>
) -> Element {
    let has_uncommitted = version_control.read().has_changes();

    if !has_uncommitted {
        return rsx! { div {} };
    }

    rsx! {
        div { class: "p-4 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-md",
            div { class: "flex items-center justify-between",
                div {
                    div { class: "text-sm font-medium text-orange-900 dark:text-orange-100",
                        "Uncommitted Changes Detected"
                    }
                    div { class: "text-xs text-orange-600 dark:text-orange-400",
                        "Clean up your workspace before proceeding"
                    }
                }
                button {
                    class: "px-3 py-1 bg-orange-600 text-white rounded-md text-sm hover:bg-orange-700",
                    onclick: move |_| show_cleanup_dialog.set(true),
                    "Cleanup"
                }
            }
        }
    }
}

#[component]
fn CleanupDialog(
    version_control: Signal<crate::VersionControlState>,
    on_close: EventHandler<MouseEvent>
) -> Element {
    let mut cleanup_options = use_signal(|| CleanupOptions {
        discard_unstaged: false,
        discard_staged: false,
        discard_untracked: true,
        create_backup: true,
    });

    rsx! {
        Modal {
            title: "Cleanup Workspace",
            visible: true,
            on_close: on_close,
            
            div { class: "space-y-4",
                div { class: "text-sm text-gray-700 dark:text-gray-300",
                    "Select which changes you want to clean up:"
                }
                
                // Cleanup options
                div { class: "space-y-3",
                    CleanupOption {
                        label: "Discard unstaged changes",
                        description: "Remove modifications to tracked files",
                        checked: cleanup_options.read().discard_unstaged,
                        on_change: move |checked| {
                            let mut opts = cleanup_options.read().clone();
                            opts.discard_unstaged = checked;
                            cleanup_options.set(opts);
                        }
                    }
                    
                    CleanupOption {
                        label: "Discard staged changes",
                        description: "Unstage all staged files",
                        checked: cleanup_options.read().discard_staged,
                        on_change: move |checked| {
                            let mut opts = cleanup_options.read().clone();
                            opts.discard_staged = checked;
                            cleanup_options.set(opts);
                        }
                    }
                    
                    CleanupOption {
                        label: "Remove untracked files",
                        description: "Delete files not tracked by Git",
                        checked: cleanup_options.read().discard_untracked,
                        on_change: move |checked| {
                            let mut opts = cleanup_options.read().clone();
                            opts.discard_untracked = checked;
                            cleanup_options.set(opts);
                        }
                    }
                    
                    CleanupOption {
                        label: "Create backup",
                        description: "Save current state before cleanup",
                        checked: cleanup_options.read().create_backup,
                        on_change: move |checked| {
                            let mut opts = cleanup_options.read().clone();
                            opts.create_backup = checked;
                            cleanup_options.set(opts);
                        }
                    }
                }
                
                // Action buttons
                div { class: "flex space-x-3 pt-4",
                    button {
                        class: "flex-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-4 py-2 rounded-md text-sm font-medium hover:bg-gray-200 dark:hover:bg-gray-700",
                        onclick: move |_| on_close.call(MouseEvent::default()),
                        "Cancel"
                    }
                    button {
                        class: "flex-1 bg-orange-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-orange-700",
                        onclick: move |_| {
                            let options = cleanup_options.read().clone();
                            let version_control = version_control.clone();
                            spawn(async move {
                                if let Some(ref conflict_resolver) = version_control.read().conflict_resolver {
                                    match conflict_resolver.cleanup_workspace(options) {
                                        Ok(cleaned_files) => {
                                            tracing::info!("Cleaned {} files", cleaned_files.len());
                                            let _ = version_control.write().refresh_status();
                                        }
                                        Err(e) => tracing::error!("Cleanup failed: {}", e),
                                    }
                                }
                            });
                            on_close.call(MouseEvent::default());
                        },
                        "🧹 Cleanup"
                    }
                }
            }
        }
    }
}

#[component]
fn CleanupOption(
    label: String,
    description: String,
    checked: bool,
    on_change: EventHandler<bool>
) -> Element {
    rsx! {
        label { class: "flex items-start space-x-3 cursor-pointer",
            input {
                r#type: "checkbox",
                class: "mt-1",
                checked: checked,
                onchange: move |evt| on_change.call(evt.checked())
            }
            div {
                div { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{label}"
                }
                div { class: "text-xs text-gray-500 dark:text-gray-400",
                    "{description}"
                }
            }
        }
    }
}

fn get_conflict_icon(conflict_type: &ConflictType) -> &'static str {
    match conflict_type {
        ConflictType::Content => "⚔️",
        ConflictType::ModifyDelete => "🗑️",
        ConflictType::DeleteModify => "📝",
        ConflictType::AddAdd => "➕",
    }
}

fn get_conflict_description(conflict_type: &ConflictType) -> &'static str {
    match conflict_type {
        ConflictType::Content => "Content conflict - both sides modified",
        ConflictType::ModifyDelete => "One side modified, other deleted",
        ConflictType::DeleteModify => "One side deleted, other modified",
        ConflictType::AddAdd => "Both sides added different content",
    }
}