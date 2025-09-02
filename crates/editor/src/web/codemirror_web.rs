use latex_ide_ui::*;
use crate::codemirror::{CodeMirrorProps, CodeMirrorOps};
#[cfg(target_arch = "wasm32")]
use crate::codemirror::DecorationType;
#[cfg(target_arch = "wasm32")]
use crate::codemirror::wrapper::{CodeMirrorEditor as Editor};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen_futures::spawn_local,
    web_sys,
    js_sys,
    std::rc::Rc,
    gloo_timers,
};

#[component]
pub fn WebCodeMirrorEditor(mut props: CodeMirrorProps) -> Element {
    let container_id = use_signal(|| format!("cm-editor-{}", uuid::Uuid::new_v4().simple()));
    
    #[cfg(target_arch = "wasm32")]
    let mut editor_instance = use_signal(|| None::<Rc<Editor>>);
    
    #[cfg(not(target_arch = "wasm32"))]
    let editor_instance = use_signal(|| None::<()>);
    
    let mut initialization_error = use_signal(|| None::<String>);
    
    // Initialize CodeMirror 6 with robust error handling
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let container_id_clone = container_id.read().clone();
            let initial_content = props.content.read().clone();
            
            spawn_local(async move {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(element) = document.get_element_by_id(&container_id_clone) {
                            match Editor::new(element, Some(initial_content)).await {
                                Ok(editor) => {
                                    let editor_rc = Rc::new(editor);
                                    editor_instance.set(Some(editor_rc.clone()));
                                    initialization_error.set(None);
                                    
                                    // Set up content change listener if callback provided
                                    if let Some(on_change) = &props.on_change {
                                        let on_change_clone = on_change.clone();
                                        let editor_clone = editor_rc.clone();
                                        
                                        // Set up debounced content checking for auto-save
                                        let mut last_content = props.content.read().clone();
                                        let mut last_change_time = None::<js_sys::Date>;
                                        spawn_local(async move {
                                            loop {
                                                gloo_timers::future::sleep(std::time::Duration::from_millis(1000)).await; // Check every 1 second
                                                let current_content = editor_clone.get_content();
                                                
                                                if current_content != last_content {
                                                    last_change_time = Some(js_sys::Date::new_0());
                                                    last_content = current_content.clone();
                                                } else if let Some(change_time) = &last_change_time {
                                                    // If content hasn't changed for 2 seconds after last change, trigger save
                                                    let now = js_sys::Date::new_0();
                                                    let time_since_change = now.get_time() - change_time.get_time();
                                                    
                                                    if time_since_change >= 2000.0 { // 2 seconds debounce
                                                        last_change_time = None;
                                                        on_change_clone.call(last_content.clone());
                                                        tracing::info!("Auto-save triggered after 2s delay");
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    
                                    tracing::info!("CodeMirror editor initialized successfully");
                                },
                                Err(e) => {
                                    let error_msg = format!("Failed to initialize editor: {}", e);
                                    tracing::error!("{}", error_msg);
                                    initialization_error.set(Some(error_msg));
                                }
                            }
                        } else {
                            let error_msg = format!("Container element '{}' not found", container_id_clone);
                            tracing::error!("{}", error_msg);
                            initialization_error.set(Some(error_msg));
                        }
                    }
                }
            });
        }
    });
    
    // Sync content changes from external signals
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        if let Some(editor) = editor_instance.read().as_ref() {
            let new_content = props.content.read();
            let current_content = editor.get_content();
            
            if current_content != *new_content {
                editor.set_content(&new_content);
            }
        }
    });
    
    // Handle format button click
    let handle_format = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(editor) = editor_instance.read().as_ref() {
            editor.format();
            
            // Update the content signal with the formatted content
            if let Some(on_change) = &props.on_change {
                let formatted_content = editor.get_content();
                props.content.set(formatted_content.clone());
                on_change.call(formatted_content);
            }
        }
    };
    
    // Handle AI suggestion button click
    let handle_ai_suggestion = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(editor) = editor_instance.read().as_ref() {
            // Example: Add a decoration to show AI suggestion
            let decoration_id = editor.add_decoration(
                10, 
                20, 
                DecorationType::Addition
            );
            
            tracing::info!("Added AI suggestion decoration: {}", decoration_id);
            
            // In a real implementation, this would trigger AI suggestion logic
            // For now, just show we have access to the robust API
        }
    };
    
    // Handle PDF sync button click
    let handle_pdf_sync = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(editor) = editor_instance.read().as_ref() {
            // Example: Highlight line for PDF sync
            editor.highlight_line(5);
            tracing::info!("PDF sync highlight applied");
        }
    };
    
    rsx! {
        div {
            class: format!("h-full flex flex-col bg-white dark:bg-gray-900 {}", 
                props.class.as_ref().unwrap_or(&String::new())),
            
            // Error display
            if let Some(error) = initialization_error.read().as_ref() {
                div {
                    class: "bg-red-50 border border-red-200 text-red-700 px-4 py-2 text-sm",
                    "⚠️ Editor Error: {error}"
                }
            }
            
            // Toolbar with enhanced functionality
            div {
                class: "flex items-center gap-2 p-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800",
                
                // Save button with status indication
                button {
                    class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                        if editor_instance.read().is_some() {
                            "bg-green-500 text-white hover:bg-green-600 focus:ring-green-500"
                        } else {
                            "bg-gray-300 text-gray-500 cursor-not-allowed"
                        }
                    ),
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(editor) = editor_instance.read().as_ref() {
                            let content = editor.get_content();
                            
                            // Update the content signal
                            props.content.set(content.clone());
                            
                            // Call the on_change callback if provided (this will trigger save and compilation)
                            if let Some(on_change) = &props.on_change {
                                on_change.call(content);
                            }
                            
                            tracing::info!("Manual save triggered");
                        }
                    },
                    disabled: editor_instance.read().is_none(),
                    "💾 Save"
                }
                
                // Format button with status indication
                button {
                    class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                        if editor_instance.read().is_some() {
                            "bg-blue-500 text-white hover:bg-blue-600 focus:ring-blue-500"
                        } else {
                            "bg-gray-300 text-gray-500 cursor-not-allowed"
                        }
                    ),
                    onclick: handle_format,
                    disabled: editor_instance.read().is_none(),
                    "📝 Format"
                }
                
                if props.enable_ai_suggestions {
                    button {
                        class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                            if editor_instance.read().is_some() {
                                "bg-green-500 text-white hover:bg-green-600 focus:ring-green-500"
                            } else {
                                "bg-gray-300 text-gray-500 cursor-not-allowed"
                            }
                        ),
                        onclick: handle_ai_suggestion,
                        disabled: editor_instance.read().is_none(),
                        "🤖 AI Suggest"
                    }
                }
                
                if props.enable_pdf_sync {
                    button {
                        class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                            if editor_instance.read().is_some() {
                                "bg-purple-500 text-white hover:bg-purple-600 focus:ring-purple-500"
                            } else {
                                "bg-gray-300 text-gray-500 cursor-not-allowed"
                            }
                        ),
                        onclick: handle_pdf_sync,
                        disabled: editor_instance.read().is_none(),
                        "🔗 Sync PDF"
                    }
                }
                
                // Editor status indicator
                div {
                    class: "ml-auto flex items-center text-xs text-gray-500",
                    match (editor_instance.read().as_ref(), initialization_error.read().as_ref()) {
                        (Some(_editor), None) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-green-100 text-green-800",
                                "● Ready"
                            }
                        },
                        (None, Some(_)) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-red-100 text-red-800",
                                "● Error"
                            }
                        },
                        (None, None) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-yellow-100 text-yellow-800",
                                "● Loading..."
                            }
                        },
                        (Some(_), Some(_)) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-orange-100 text-orange-800",
                                "● Warning"
                            }
                        },
                    }
                }
            }
            
            // Editor container with loading/error states
            div {
                class: "flex-1 relative",
                
                // Main editor container
                div {
                    id: "{container_id.read()}",
                    class: "h-full w-full",
                    
                    // Loading overlay
                    if editor_instance.read().is_none() && initialization_error.read().is_none() {
                        div {
                            class: "absolute inset-0 flex items-center justify-center bg-gray-50 dark:bg-gray-800 bg-opacity-75",
                            div {
                                class: "text-center",
                                div {
                                    class: "inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mb-4",
                                }
                                div {
                                    class: "text-sm text-gray-600 dark:text-gray-400",
                                    "Initializing CodeMirror 6..."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}