use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable, GlobalSignal};
use dioxus_hooks::use_signal;
use crate::desktop::DesktopCodeMirrorEditor;

// Re-export the EditorViewMode from the web version for consistency
pub use crate::web::EditorViewMode;

#[component]
pub fn ChatCodeToggle(
    mut view_mode: Signal<EditorViewMode>,
    mut show_floating_chat: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "flex items-center space-x-2",
            // Chat/Code Switch
            div { class: "flex items-center bg-zinc-100 dark:bg-zinc-800 rounded-lg p-1",
                button {
                    class: if *view_mode.read() == EditorViewMode::Code {
                        "px-3 py-1 text-xs font-medium text-white bg-zinc-900 dark:bg-zinc-100 dark:text-zinc-900 rounded-md transition-colors"
                    } else {
                        "px-3 py-1 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                    },
                    onclick: move |_| {
                        view_mode.set(EditorViewMode::Code);
                        // When switching to code mode, automatically show floating chat if it was hidden
                        if !*show_floating_chat.read() {
                            show_floating_chat.set(true);
                        }
                    },
                    "Code"
                }
                button {
                    class: if *view_mode.read() == EditorViewMode::Chat {
                        "px-3 py-1 text-xs font-medium text-white bg-zinc-900 dark:bg-zinc-100 dark:text-zinc-900 rounded-md transition-colors"
                    } else {
                        "px-3 py-1 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                    },
                    onclick: move |_| {
                        view_mode.set(EditorViewMode::Chat);
                        // Hide floating chat when in full chat mode
                        show_floating_chat.set(false);
                    },
                    "Chat"
                }
            }
            
            // Floating chat toggle (only visible in Code mode)
            if *view_mode.read() == EditorViewMode::Code {
                button {
                    class: if *show_floating_chat.read() {
                        "p-1.5 rounded-lg bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400 transition-colors"
                    } else {
                        "p-1.5 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 dark:text-zinc-400 transition-colors"
                    },
                    onclick: move |_| {
                        let current = *show_floating_chat.read();
                        show_floating_chat.set(!current);
                    },
                    title: if *show_floating_chat.read() { "Hide Floating Chat" } else { "Show Floating Chat" },
                    
                    // Chat bubble icon
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path { d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DesktopEditorPane(
    document_content: Signal<String>,
    current_file: Signal<Option<String>>
) -> Element {
    let view_mode = use_signal(|| EditorViewMode::Code);
    let show_floating_chat = use_signal(|| false);
    
    let current_file_guard = current_file.read();
    let file_name = current_file_guard.as_ref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("No file selected");
    
    rsx! {
        div { class: "h-full flex flex-col bg-white dark:bg-zinc-950",
            
            // Header with file name and Chat/Code toggle
            div { class: "h-10 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 flex items-center justify-between px-4",
                // Left side - File name
                div { class: "flex items-center space-x-3",
                    if current_file.read().is_some() {
                        div { class: "text-sm font-medium text-zinc-900 dark:text-zinc-100",
                            "{file_name}"
                        }
                    } else {
                        div { class: "text-sm text-zinc-500 dark:text-zinc-400",
                            "No file selected"
                        }
                    }
                }
                
                // Right side - Chat/Code toggle
                ChatCodeToggle {
                    view_mode: view_mode,
                    show_floating_chat: show_floating_chat,
                }
            }
            
            // Main content area - switches between Chat and Code modes
            div { class: "flex-1 overflow-hidden relative",
                match *view_mode.read() {
                    EditorViewMode::Chat => {
                        rsx! {
                            div { class: "h-full flex items-center justify-center text-zinc-500 dark:text-zinc-400",
                                div { class: "text-center",
                                    div { class: "text-4xl mb-4", "🤖" }
                                    div { class: "text-lg mb-2", "AI Chat Mode" }
                                    div { class: "text-sm", "Chat interface will be available when desktop chat components are fully implemented" }
                                }
                            }
                        }
                    }
                    EditorViewMode::Code => {
                        if current_file.read().is_some() {
                            rsx! {
                                div { class: "h-full relative",
                                    // Desktop CodeMirror Editor
                                    div { class: "h-full",
                                        DesktopCodeMirrorEditor {
                                            content: document_content,
                                            enable_ai_suggestions: true,
                                            enable_pdf_sync: true,
                                            on_change: Some(EventHandler::new(move |new_content: String| {
                                                // Update the document content signal
                                                document_content.set(new_content.clone());
                                                
                                                // Auto-save to desktop filesystem when content changes
                                                if let Some(file_path) = current_file.read().as_ref() {
                                                    let file_path_clone = file_path.clone();
                                                    
                                                    // For desktop, we can directly write to filesystem
                                                    std::thread::spawn(move || {
                                                        if let Err(e) = std::fs::write(&file_path_clone, &new_content) {
                                                            tracing::error!("Failed to auto-save file {}: {}", file_path_clone, e);
                                                        } else {
                                                            tracing::info!("Auto-saved file: {}", file_path_clone);
                                                            
                                                            // Trigger compilation if it's a .tex file
                                                            if file_path_clone.ends_with(".tex") {
                                                                // For desktop, we can use system LaTeX compiler
                                                                let document_dir = std::path::Path::new(&file_path_clone).parent().unwrap_or(std::path::Path::new("."));
                                                                let compile_result = std::process::Command::new("pdflatex")
                                                                    .arg("-interaction=nonstopmode")
                                                                    .arg(&file_path_clone)
                                                                    .current_dir(document_dir)
                                                                    .output();
                                                                    
                                                                match compile_result {
                                                                    Ok(output) => {
                                                                        if output.status.success() {
                                                                            tracing::info!("LaTeX compilation successful for: {}", file_path_clone);
                                                                        } else {
                                                                            let stderr = String::from_utf8_lossy(&output.stderr);
                                                                            tracing::error!("LaTeX compilation failed for {}: {}", file_path_clone, stderr);
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        tracing::error!("Failed to run LaTeX compiler: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    });
                                                }
                                            })),
                                        }
                                    }
                                    
                                    // Floating chat panel at bottom when enabled (placeholder for now)
                                    if *show_floating_chat.read() {
                                        div { 
                                            class: "absolute bottom-4 right-4 w-80 h-32 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-lg shadow-lg z-10",
                                            div { class: "p-3 text-center text-zinc-500 dark:text-zinc-400",
                                                div { class: "text-sm", "💬 Floating Chat" }
                                                div { class: "text-xs mt-1", "Coming soon for desktop" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "h-full flex items-center justify-center",
                                    div { class: "text-center",
                                        svg {
                                            class: "w-16 h-16 mx-auto mb-4 text-zinc-300 dark:text-zinc-700",
                                            fill: "none",
                                            stroke: "currentColor",
                                            stroke_width: "2",
                                            view_box: "0 0 24 24",
                                            path {
                                                d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                            }
                                            polyline {
                                                points: "14 2 14 8 20 8"
                                            }
                                            line { x1: "16", y1: "13", x2: "8", y2: "13" }
                                            line { x1: "16", y1: "17", x2: "8", y2: "17" }
                                            polyline {
                                                points: "10 9 9 9 8 9"
                                            }
                                        }
                                        div { class: "text-lg font-medium mb-2 text-zinc-900 dark:text-zinc-100", "No File Selected" }
                                        div { class: "text-sm text-zinc-500 dark:text-zinc-400", "Choose a file from the workspace to start editing" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}