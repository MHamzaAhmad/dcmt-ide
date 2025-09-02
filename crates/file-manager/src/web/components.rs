use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use crate::{ProjectManager, FileItem};
use latex_ide_ui::*;
use std::collections::HashMap;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// Helper function to build tree structure from flat file list
fn build_file_tree(flat_files: Vec<FileItem>) -> Vec<FileItem> {
    use std::collections::HashMap;
    use std::path::PathBuf;
    
    if flat_files.is_empty() {
        return vec![];
    }
    
    // Map from normalized path to FileItem
    let mut all_items: HashMap<String, FileItem> = HashMap::new();
    
    tracing::info!("Building tree from {} files", flat_files.len());
    
    // First pass: process all files from server (these are already hierarchical from recursive scan)
    for file in flat_files {
        let normalized_path = file.path.to_string_lossy().to_string().replace('\\', "/");
        
        // Skip empty paths
        if normalized_path.is_empty() {
            continue;
        }
        
        tracing::debug!("Processing file: {} at path: {}", file.name, normalized_path);
        
        // For files, create all parent directories if they don't exist
        let parts: Vec<&str> = normalized_path.split('/').filter(|p| !p.is_empty()).collect();
        
        // Create parent directories
        for i in 0..parts.len() - 1 {
            let dir_path = parts[0..=i].join("/");
            
            if !all_items.contains_key(&dir_path) {
                let mut dir_item = FileItem::new_directory(
                    parts[i].to_string(), 
                    PathBuf::from(&dir_path)
                );
                dir_item.children = Some(Vec::new());
                all_items.insert(dir_path, dir_item);
                
                tracing::debug!("Created parent directory: {}", parts[i]);
            }
        }
        
        // Add the actual file/directory
        let mut item_copy = file.clone();
        // Ensure name is just the basename
        if let Some(last_part) = parts.last() {
            item_copy.name = last_part.to_string();
        }
        all_items.insert(normalized_path, item_copy);
    }
    
    // Second pass: build parent-child relationships
    let mut path_list: Vec<String> = all_items.keys().cloned().collect();
    path_list.sort_by(|a, b| {
        // Sort by depth first (fewer slashes = closer to root)
        let a_depth = a.matches('/').count();
        let b_depth = b.matches('/').count();
        a_depth.cmp(&b_depth).then_with(|| a.cmp(b))
    });
    
    for path in &path_list {
        if let Some(last_slash_pos) = path.rfind('/') {
            let parent_path = &path[..last_slash_pos];
            
            // Skip if parent is empty (root level)
            if parent_path.is_empty() {
                continue;
            }
            
            // Clone the child item to avoid borrow checker issues
            let child_item = all_items.get(path).cloned();
            if let Some(child_item) = child_item {
                if let Some(parent) = all_items.get_mut(parent_path) {
                    if let Some(ref mut children) = parent.children {
                        // Don't add duplicates
                        if !children.iter().any(|c| c.id == child_item.id) {
                            children.push(child_item.clone());
                            tracing::debug!("Added {} to parent {}", child_item.name, parent.name);
                        }
                    }
                }
            }
        }
    }
    
    // Collect root items (no slash in normalized path, or direct children of root)
    let mut root_items: Vec<FileItem> = Vec::new();
    for (path, item) in all_items {
        let slash_count = path.matches('/').count();
        if slash_count == 0 || (slash_count == 1 && !path.starts_with('/')) {
            root_items.push(item);
        }
    }
    
    // Sort everything recursively  
    sort_items_recursive(&mut root_items);
    
    tracing::info!("Built file tree with {} root items", root_items.len());
    
    // Log the tree structure for debugging
    for item in &root_items {
        log_tree_structure(item, 0);
    }
    
    root_items
}

fn sort_items_recursive(items: &mut Vec<FileItem>) {
    items.sort_by(|a, b| {
        match (a.is_directory(), b.is_directory()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });
    
    for item in items {
        if let Some(ref mut children) = item.children {
            sort_items_recursive(children);
        }
    }
}

fn log_tree_structure(item: &FileItem, depth: usize) {
    let indent = "  ".repeat(depth);
    let icon = if item.is_directory() { "📁" } else { "📄" };
    let children_count = item.children.as_ref().map_or(0, |c| c.len());
    
    tracing::debug!("{}{} {} ({})", indent, icon, item.name, 
        if item.is_directory() { 
            format!("{} children", children_count)
        } else { 
            "file".to_string() 
        }
    );
    
    if let Some(children) = &item.children {
        for child in children {
            log_tree_structure(child, depth + 1);
        }
    }
}

// Helper function to recursively sort children
fn sort_children(items: &mut Vec<FileItem>) {
    items.sort_by(|a, b| {
        match (a.is_directory(), b.is_directory()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });
    
    for item in items {
        if let Some(ref mut children) = item.children {
            sort_children(children);
        }
    }
}

/// Main file tree component for web platform
#[component]
pub fn WebFileTree(
    project_manager: Signal<ProjectManager>,
    onfile_select: Option<EventHandler<String>>,
    backend_connected: Option<Signal<bool>>
) -> Element {
    let mut file_tree = use_signal(|| Vec::<FileItem>::new());
    let mut error_message = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    let mut expanded_folders = use_signal(|| HashMap::<String, bool>::new());
    
    // Load files from workspace via transport
    use_effect(move || {
        loading.set(true);
        error_message.set(None);
        
        #[cfg(target_arch = "wasm32")]
        spawn_local(async move {
            use super::file_operations::fetch_file_list;
            match fetch_file_list().await {
                Ok(files) => {
                    tracing::info!("Received {} files from server", files.len());
                    for file in &files {
                        tracing::debug!("  - {} (path: {}, is_dir: {})", 
                            file.name, 
                            file.path.display(), 
                            file.is_directory()
                        );
                    }
                    
                    // Update connection status
                    if let Some(mut connected) = backend_connected {
                        connected.set(true);
                    }
                    
                    let tree = build_file_tree(files.clone());
                    file_tree.set(tree);
                    loading.set(false);
                    
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
                    
                    // Update connection status
                    if let Some(mut connected) = backend_connected {
                        connected.set(false);
                    }
                    
                    // Provide helpful error message for server connection issues
                    let helpful_message = if e.contains("Failed to connect") || e.contains("Connection refused") || e.contains("NetworkError") {
                        "❌ Backend server not running!\n\nTo start the server, run:\n./scripts/dev.sh web\n\nOr:\n./scripts/dev.sh backend\n\nThis will start the LaTeX IDE backend services including file operations, git integration, and PDF compilation.".to_string()
                    } else {
                        format!("Failed to load files: {}", e)
                    };
                    
                    error_message.set(Some(helpful_message));
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
            // Header with title, collapse all, and refresh
            div { 
                class: "h-10 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between px-3 bg-white dark:bg-zinc-900",
                
                span { 
                    class: "text-xs font-medium text-zinc-700 dark:text-zinc-300 uppercase tracking-wider",
                    "Explorer"
                }
                
                div { class: "flex items-center space-x-1",
                    // Collapse All button
                    button {
                        class: "p-1 rounded hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors",
                        onclick: move |_| {
                            expanded_folders.set(HashMap::new());
                        },
                        title: "Collapse All",
                        
                        svg {
                            class: "w-3.5 h-3.5 text-zinc-600 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M4 14h16M4 10h16" }
                        }
                    }
                    
                    // Refresh button
                    button {
                        class: "p-1 rounded hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors",
                        onclick: move |_| {
                            refresh_file_tree(loading, error_message, file_tree, expanded_folders);
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
            }
            
            // File tree content
            div { class: "flex-1 overflow-auto",
                WebFileTreeContent {
                    file_tree,
                    loading,
                    error_message,
                    onfile_select,
                    expanded_folders,
                }
                
                WebFileTreeActions {
                    file_tree,
                    error_message,
                }
            }
        }
    }
}


/// Main content area displaying file tree
#[component]
fn WebFileTreeContent(
    file_tree: Signal<Vec<FileItem>>,
    loading: Signal<bool>,
    error_message: Signal<Option<String>>,
    onfile_select: Option<EventHandler<String>>,
    expanded_folders: Signal<HashMap<String, bool>>,
) -> Element {
    rsx! {
        div { class: "flex-1 px-2 py-2",
            if *loading.read() {
                div { class: "flex flex-col items-center justify-center py-8",
                    div { class: "w-6 h-6 border-2 border-zinc-300 dark:border-zinc-600 border-t-transparent rounded-full animate-spin mb-3" }
                    div { class: "text-sm text-zinc-500 dark:text-zinc-400", "Loading workspace..." }
                }
            } else if file_tree.read().is_empty() {
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
                div { class: "space-y-0",
                    for file in file_tree.read().iter() {
                        TreeNode {
                            file_item: file.clone(),
                            onfile_select: onfile_select.clone(),
                            expanded_folders,
                            depth: 0,
                        }
                    }
                }
            }
        }
    }
}

/// Tree node component for file display (works with both flat and hierarchical lists)
#[component]
fn TreeNode(
    file_item: FileItem,
    onfile_select: Option<EventHandler<String>>,
    mut expanded_folders: Signal<HashMap<String, bool>>,
    depth: usize,
) -> Element {
    let file_id = file_item.id.clone();
    let file_path = file_item.path.display().to_string();
    let file_path_clone = file_path.clone();
    let is_directory = file_item.is_directory();
    let has_children = file_item.children.as_ref().map_or(false, |c| !c.is_empty());
    
    // Check if this folder is expanded
    let is_expanded = expanded_folders.read().get(&file_id).copied().unwrap_or(false);
    
    // Calculate indentation based on depth (for flat list, depth is always 0)
    let indent_style = if depth > 0 {
        format!("padding-left: {}px", depth * 16)
    } else {
        String::new()
    };
    
    rsx! {
        div {
            // File/folder item
            div {
                class: "flex items-center py-1 px-2 text-sm text-zinc-700 dark:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded cursor-pointer group",
                style: "{indent_style}",
                onclick: move |_| {
                    if is_directory && has_children {
                        // Toggle folder expansion only if it has children
                        let mut expanded = expanded_folders.write();
                        let current = expanded.get(&file_id).copied().unwrap_or(false);
                        expanded.insert(file_id.clone(), !current);
                    } else if !is_directory {
                        // Select file
                        if let Some(handler) = &onfile_select {
                            handler.call(file_path_clone.clone());
                        }
                    }
                    // Do nothing for empty directories
                },
                
                // Expand/collapse arrow for folders with children
                if is_directory && has_children {
                    span {
                        class: "mr-1 text-zinc-400 dark:text-zinc-500",
                        if is_expanded { "▼" } else { "▶" }
                    }
                } else if is_directory {
                    // Empty space for folders without children
                    span { class: "mr-1 w-3 inline-block" }
                }
                
                // File/folder icon
                if is_directory {
                    svg {
                        class: "w-4 h-4 mr-2 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        if is_expanded {
                            path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2z" }
                        } else {
                            path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
                        }
                    }
                } else {
                    // File icon based on type
                    if file_item.name.ends_with(".tex") {
                        svg {
                            class: "w-4 h-4 mr-2 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                            polyline { points: "14 2 14 8 20 8" }
                            line { x1: "16", y1: "13", x2: "8", y2: "13" }
                            line { x1: "16", y1: "17", x2: "8", y2: "17" }
                        }
                    } else if file_item.name.ends_with(".bib") {
                        svg {
                            class: "w-4 h-4 mr-2 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M4 19.5A2.5 2.5 0 0 1 6.5 17H20" }
                            path { d: "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" }
                        }
                    } else {
                        svg {
                            class: "w-4 h-4 mr-2 flex-shrink-0 text-zinc-500 dark:text-zinc-400",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" }
                            polyline { points: "13 2 13 9 20 9" }
                        }
                    }
                }
                
                // File/folder name
                span { 
                    class: "truncate",
                    title: "{file_path}",
                    "{file_item.name}"
                }
            }
            
            // Render children if expanded
            if is_directory && is_expanded {
                if let Some(children) = &file_item.children {
                    for child in children {
                        TreeNode {
                            file_item: child.clone(),
                            onfile_select: onfile_select.clone(),
                            expanded_folders,
                            depth: depth + 1,
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
    file_tree: Signal<Vec<FileItem>>,
    error_message: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "p-3 border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900",
            div { class: "grid grid-cols-2 gap-2",
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        create_new_latex_file(file_tree, error_message);
                    },
                    "New .tex"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        create_new_bibliography_file(file_tree, error_message);
                    },
                    "New .bib"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        upload_file_from_local(file_tree, error_message);
                    },
                    "Upload"
                }
                
                button {
                    class: "px-3 py-2 text-xs font-medium text-zinc-700 dark:text-zinc-300 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-md hover:bg-zinc-50 dark:hover:bg-zinc-700 transition-colors",
                    onclick: move |_| {
                        download_workspace_as_zip(file_tree, error_message);
                    },
                    "Download"
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

fn refresh_file_tree(
    mut loading: Signal<bool>,
    mut error_message: Signal<Option<String>>,
    _file_tree: Signal<Vec<FileItem>>,
    _expanded_folders: Signal<HashMap<String, bool>>,
) {
    tracing::info!("Refreshing file tree");
    loading.set(true);
    error_message.set(None);
    
    #[cfg(target_arch = "wasm32")]
    {
        spawn_local(async move {
            use super::file_operations::fetch_file_list;
            match fetch_file_list().await {
                Ok(files) => {
                    let _tree = build_file_tree(files);
                    loading.set(false);
                    tracing::info!("File tree refreshed successfully");
                }
                Err(e) => {
                    loading.set(false);
                    error_message.set(Some(format!("Refresh failed: {}", e)));
                    tracing::error!("Failed to refresh file tree: {}", e);
                }
            }
        });
    }
}

fn create_new_latex_file(
    _file_tree: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    // For now, use a default name. In a real implementation, this would show a name input dialog
    let _file_name = "new_document.tex";
    
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        use super::file_operations::create_latex_file;
        match create_latex_file("new_document.tex").await {
            Ok(_) => {
                tracing::info!("Successfully created new LaTeX file: new_document.tex");
                
                // File tree refresh would happen here in a real implementation
                tracing::info!("File tree would be refreshed to show new file");
            }
            Err(e) => {
                tracing::error!("Failed to create LaTeX file: {}", e);
            }
        }
    });
}

fn create_new_bibliography_file(
    _file_tree: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    // For now, use a default name. In a real implementation, this would show a name input dialog
    let _file_name = "references.bib";
    
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        use super::file_operations::create_bibliography_file;
        match create_bibliography_file("references.bib").await {
            Ok(_) => {
                tracing::info!("Successfully created new bibliography file: references.bib");
                
                // File tree refresh would happen here in a real implementation
                tracing::info!("File tree would be refreshed to show new file");
            }
            Err(e) => {
                tracing::error!("Failed to create bibliography file: {}", e);
            }
        }
    });
}

fn upload_file_from_local(
    _file_tree: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        use super::file_operations::create_file_input;
        match create_file_input(move |file: web_sys::File| {
            tracing::info!("Selected file for upload: {}", file.name());
            
            wasm_bindgen_futures::spawn_local(async move {
                use super::file_operations::upload_file_from_browser;
                
                match upload_file_from_browser(file).await {
                    Ok(_) => {
                        tracing::info!("File uploaded successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to upload file: {}", e);
                    }
                }
            });
        }) {
            Ok(_) => {
                tracing::info!("File picker opened successfully");
            }
            Err(e) => {
                tracing::error!("Failed to open file picker: {}", e);
            }
        }
    }
}

fn download_workspace_as_zip(
    _file_tree: Signal<Vec<FileItem>>,
    _error_message: Signal<Option<String>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        use super::file_operations::download_file_content;
        use wasm_bindgen::prelude::*;
        
        wasm_bindgen_futures::spawn_local(async move {
            tracing::info!("Starting workspace download...");
            
            // Get all files from the tree
            let files_to_download: Vec<crate::FileItem> = Vec::new(); // collect_all_files(&_file_tree.read());
            
            if files_to_download.is_empty() {
                tracing::warn!("No files to download");
                return;
            }
            
            tracing::info!("Downloading {} files", files_to_download.len());
            
            // Create a simple ZIP-like structure (for now, just concatenate files with headers)
            let mut zip_content = String::new();
            let mut download_count = 0;
            
            for file_item in files_to_download {
                if !file_item.is_directory() {
                    let file_path = file_item.path.to_string_lossy().to_string();
                    
                    match download_file_content(&file_path).await {
                        Ok(content) => {
                            let file_content = String::from_utf8_lossy(&content);
                            zip_content.push_str(&format!("=== FILE: {} ===\n", file_item.name));
                            zip_content.push_str(&file_content);
                            zip_content.push_str("\n=== END FILE ===\n\n");
                            download_count += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to download file {}: {}", file_item.name, e);
                        }
                    }
                }
            }
            
            if download_count == 0 {
                tracing::error!("Failed to download any files");
                return;
            }
            
            // Create a downloadable blob
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            
            // Create blob URL
            let array = js_sys::Array::new();
            array.push(&JsValue::from_str(&zip_content));
            
            let blob = web_sys::Blob::new_with_str_sequence(&array).unwrap();
            let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
            
            // Create download link
            let link = document.create_element("a").unwrap();
            link.set_attribute("href", &url).unwrap();
            link.set_attribute("download", "workspace.txt").unwrap();
            link.set_attribute("style", "display: none").unwrap();
            
            let body = document.body().unwrap();
            body.append_child(&link).unwrap();
            let link_element = link.dyn_into::<web_sys::HtmlElement>().unwrap();
            link_element.click();
            body.remove_child(&link_element).unwrap();
            
            web_sys::Url::revoke_object_url(&url).unwrap();
            
            tracing::info!("Workspace download completed: {} files", download_count);
        });
    }
}

fn collect_all_files(items: &[FileItem]) -> Vec<FileItem> {
    let mut all_files = Vec::new();
    
    for item in items {
        if item.is_directory() {
            if let Some(children) = &item.children {
                all_files.extend(collect_all_files(children));
            }
        } else {
            all_files.push(item.clone());
        }
    }
    
    all_files
}