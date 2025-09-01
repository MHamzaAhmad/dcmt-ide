use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::use_signal;
use crate::{ChatEngine, ChatMessage};
use latex_ide_ui::*;
use latex_ide_ui::button::ButtonVariant;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
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
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // AI Chat header
            div { class: "p-4 border-b border-gray-200 dark:border-gray-700",
                h3 { class: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-3",
                    "AI Assistant"
                }
                
                // Model selection
                Dropdown {
                    items: chat_engine.read().available_models.iter().enumerate().map(|(_i, model)| {
                        DropdownItem { 
                            id: model.id.clone(), 
                            label: model.name.clone(),
                            icon: None 
                        }
                    }).collect(),
                    selected: chat_engine.read().selected_model.clone(),
                    onselect: move |model_id: String| {
                        chat_engine.write().set_selected_model(model_id);
                    },
                    placeholder: "Select Model".to_string(),
                }
                
                // Connection status
                if capabilities.read().server_sent_events {
                    div { class: "text-xs text-green-600 dark:text-green-400 mt-2",
                        "🟢 Connected via Server-Sent Events"
                    }
                } else {
                    div { class: "text-xs text-yellow-600 dark:text-yellow-400 mt-2",
                        "🟡 Limited AI connectivity"
                    }
                }
            }
            
            // Chat messages
            div { class: "flex-1 overflow-auto p-4 space-y-4",
                if chat_engine.read().messages.is_empty() {
                    div { class: "text-center text-gray-500 dark:text-gray-400 mt-8",
                        div { class: "text-4xl mb-4", "🤖" }
                        div { class: "text-lg mb-2", "AI Assistant Ready" }
                        div { class: "text-sm",
                            "Ask me about LaTeX syntax, document structure,"
                            br {}
                            "mathematical typesetting, or get writing help."
                        }
                    }
                } else {
                    for message in chat_engine.read().messages.iter() {
                        ChatBubble { message: message.clone() }
                    }
                }
                
                if chat_engine.read().is_streaming {
                    div { class: "flex items-center space-x-2 text-gray-500 dark:text-gray-400",
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full" }
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full", style: "animation-delay: 0.2s" }
                        div { class: "animate-pulse w-2 h-2 bg-blue-500 rounded-full", style: "animation-delay: 0.4s" }
                        span { class: "text-sm", "AI is thinking..." }
                    }
                }
            }
            
            // Message input
            div { class: "p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800",
                div { class: "flex space-x-2",
                    Input {
                        value: Some(current_message.read().clone()),
                        placeholder: Some("Ask about LaTeX, document structure, or get writing help...".to_string()),
                        onchange: move |value| {
                            current_message.set(value);
                        }
                    }
                    
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: chat_engine.read().is_streaming || current_message.read().trim().is_empty(),
                        onclick: move |_| {
                            let message = current_message.read().clone();
                            if !message.trim().is_empty() {
                                // Add user message
                                chat_engine.write().add_user_message(message);
                                current_message.set(String::new());
                                chat_engine.write().set_streaming(true);
                                
                                // Simulate AI response (replace with actual API call)
                                #[cfg(target_arch = "wasm32")]
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
    let bg_class = if message.is_user() {
        "bg-blue-500 text-white ml-auto"
    } else {
        "bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100 mr-auto"
    };
    
    rsx! {
        div { 
            class: "max-w-xs lg:max-w-md px-4 py-2 rounded-lg {bg_class}",
            "{message.content}"
        }
    }
}