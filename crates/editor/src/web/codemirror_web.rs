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
    let config = props.to_editor_config();
    
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
            let config_clone = config.clone();
            
            spawn_local(async move {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(element) = document.get_element_by_id(&container_id_clone) {
                            match Editor::new_with_config(element, Some(initial_content), config_clone).await {
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
    
    rsx! {
        div {
            class: format!("h-full w-full flex flex-col bg-white dark:bg-zinc-950 {}", 
                props.class.as_ref().unwrap_or(&String::new())),
            
            // Error display
            if let Some(error) = initialization_error.read().as_ref() {
                div {
                    class: "bg-red-50 dark:bg-red-900 border border-red-200 dark:border-red-700 text-red-700 dark:text-red-300 px-4 py-2 text-sm",
                    "⚠️ Editor Error: {error}"
                }
            }
            
            // Editor container with loading/error states
            div {
                class: "flex-1 relative overflow-hidden",
                
                // Main editor container
                div {
                    id: "{container_id.read()}",
                    class: "h-full w-full overflow-hidden",
                    
                    // Loading overlay
                    if editor_instance.read().is_none() && initialization_error.read().is_none() {
                        div {
                            class: "absolute inset-0 flex items-center justify-center bg-zinc-50 dark:bg-zinc-800 bg-opacity-75",
                            div {
                                class: "text-center",
                                div {
                                    class: "inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mb-4",
                                }
                                div {
                                    class: "text-sm text-zinc-600 dark:text-zinc-400",
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