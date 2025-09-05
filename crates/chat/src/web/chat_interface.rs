use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::{use_signal, use_effect};
use crate::{ChatEngine, ChatMessage};
use latex_ide_ui::*;
use latex_ide_ui::button::ButtonVariant;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen_futures::spawn_local;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use gloo_timers::future::sleep;

/// Browser capabilities for web-specific features
#[derive(Clone, Debug)]
pub struct BrowserCapabilities {
    pub server_sent_events: bool,
    pub websocket_supported: bool,
    pub webtransport_supported: bool,
}

impl Default for BrowserCapabilities {
    fn default() -> Self {
        Self {
            server_sent_events: false,
            websocket_supported: true,
            webtransport_supported: false,
        }
    }
}

#[component]
pub fn WebAIChatInterface(capabilities: Signal<BrowserCapabilities>) -> Element {
    let mut chat_engine = use_signal(|| ChatEngine::new());
    let mut current_message = use_signal(|| String::new());
    
    // Load models on first render (web only for now, desktop loads on demand)
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    use_effect(move || {
        let mut engine = chat_engine.clone();
        spawn_local(async move {
            let _ = engine.write().load_models().await;
        });
    });
    
    rsx! {
        div { class: "h-full flex flex-col bg-white dark:bg-zinc-950",
            
            // Simplified header - just title and connection status
            div { class: "px-4 py-3 border-b border-zinc-200 dark:border-zinc-800",
                h3 { class: "text-lg font-semibold text-zinc-900 dark:text-zinc-100",
                    "AI Assistant"
                }
                
                // Connection status
                if capabilities.read().server_sent_events {
                    div { class: "text-xs text-emerald-600 dark:text-emerald-400 mt-1",
                        "Connected via SSE"
                    }
                } else {
                    div { class: "text-xs text-amber-600 dark:text-amber-400 mt-1",
                        "Limited connectivity"
                    }
                }
            }
            
            // Chat messages area
            div { class: "flex-1 overflow-auto px-4 py-3 space-y-3",
                if chat_engine.read().messages.is_empty() {
                    div { class: "flex flex-col items-center justify-center h-full text-center",
                        div { class: "text-4xl mb-3 text-zinc-400 dark:text-zinc-500", "💬" }
                        div { class: "text-lg font-medium text-zinc-900 dark:text-zinc-100 mb-2", "AI Assistant Ready" }
                        div { class: "text-sm text-zinc-600 dark:text-zinc-400 max-w-sm",
                            "Ask me about LaTeX syntax, document structure, mathematical typesetting, or get writing help."
                        }
                    }
                } else {
                    for message in chat_engine.read().messages.iter() {
                        ChatBubble { message: message.clone() }
                    }
                }
                
                if chat_engine.read().is_streaming {
                    div { class: "flex items-center space-x-2 text-zinc-500 dark:text-zinc-400 py-2",
                        div { class: "flex space-x-1",
                            div { class: "w-2 h-2 bg-zinc-400 dark:bg-zinc-500 rounded-full animate-pulse" }
                            div { class: "w-2 h-2 bg-zinc-400 dark:bg-zinc-500 rounded-full animate-pulse", style: "animation-delay: 0.2s" }
                            div { class: "w-2 h-2 bg-zinc-400 dark:bg-zinc-500 rounded-full animate-pulse", style: "animation-delay: 0.4s" }
                        }
                        span { class: "text-sm", "AI is thinking..." }
                    }
                }
            }
            
            // Input area with model selection above
            div { class: "border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 p-4 space-y-3",
                
                // Model selection dropdown
                div { class: "flex items-center space-x-3",
                    span { class: "text-sm font-medium text-zinc-700 dark:text-zinc-300 whitespace-nowrap",
                        "Model:"
                    }
                    
                    if chat_engine.read().is_loading_models {
                        div { class: "text-sm text-zinc-500 dark:text-zinc-400",
                            "Loading models..."
                        }
                    } else if let Some(error) = &chat_engine.read().model_load_error {
                        div { class: "flex items-center space-x-2",
                            div { class: "text-sm text-red-600 dark:text-red-400",
                                "Error loading models"
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| {
                                    #[cfg(all(target_arch = "wasm32", feature = "web"))]
                                    {
                                        let mut engine = chat_engine.clone();
                                        spawn_local(async move {
                                            let _ = engine.write().load_models().await;
                                        });
                                    }
                                },
                                "Retry"
                            }
                        }
                    } else if chat_engine.read().available_models.is_empty() {
                        div { class: "flex items-center space-x-2",
                            div { class: "text-sm text-zinc-500 dark:text-zinc-400",
                                "No models loaded"
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                onclick: move |_| {
                                    #[cfg(all(target_arch = "wasm32", feature = "web"))]
                                    {
                                        let mut engine = chat_engine.clone();
                                        spawn_local(async move {
                                            let _ = engine.write().load_models().await;
                                        });
                                    }
                                },
                                "Load Models"
                            }
                        }
                    } else {
                        Dropdown {
                            items: chat_engine.read().available_models.iter().map(|model| {
                                DropdownItem::new(model.id.clone(), model.name.clone())
                            }).collect(),
                            selected: chat_engine.read().selected_model.clone(),
                            onselect: move |model_id: String| {
                                chat_engine.write().set_selected_model(model_id);
                            },
                            placeholder: "Select Model".to_string(),
                            class: Some("min-w-0 flex-1".to_string()),
                        }
                    }
                }
                
                // Message input row
                div { class: "flex space-x-2",
                    div { class: "flex-1",
                        Input {
                            value: Some(current_message.read().clone()),
                            placeholder: Some("Ask about LaTeX, document structure, or get writing help...".to_string()),
                            onchange: move |value| {
                                current_message.set(value);
                            }
                        }
                    }
                    
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: chat_engine.read().is_streaming || current_message.read().trim().is_empty() || chat_engine.read().selected_model.is_none(),
                        onclick: move |_| {
                            let message = current_message.read().clone();
                            if !message.trim().is_empty() && chat_engine.read().selected_model.is_some() {
                                // Add user message
                                chat_engine.write().add_user_message(message);
                                current_message.set(String::new());
                                chat_engine.write().set_streaming(true);
                                
                                // Simulate AI response (replace with actual API call)
                                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                                {
                                    let mut engine = chat_engine.clone();
                                    spawn_local(async move {
                                        sleep(std::time::Duration::from_secs(2)).await;
                                        engine.write().add_assistant_message(
                                            "I can help you with that LaTeX question! Here's what I suggest...".to_string()
                                        );
                                        engine.write().set_streaming(false);
                                    });
                                }
                            }
                        },
                        if chat_engine.read().is_streaming { "..." } else { "Send" }
                    }
                }
            }
        }
    }
}

#[component]
fn ChatBubble(message: ChatMessage) -> Element {
    let (bg_class, alignment_class) = if message.is_user() {
        (
            "bg-zinc-900 dark:bg-zinc-100 text-zinc-50 dark:text-zinc-900",
            "ml-auto"
        )
    } else {
        (
            "bg-zinc-100 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100",
            "mr-auto"
        )
    };
    
    rsx! {
        div { 
            class: "max-w-sm lg:max-w-md px-3 py-2 rounded-lg {bg_class} {alignment_class}",
            p { class: "text-sm leading-relaxed whitespace-pre-wrap",
                "{message.content}"
            }
        }
    }
}