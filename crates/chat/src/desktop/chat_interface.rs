use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable};
use dioxus_hooks::use_signal;
use crate::ChatEngine;
#[cfg(feature = "latex-ide-model-manager")]
use latex_ide_model_manager::{ModelConfig, ModelManager};
use latex_ide_ui::*;
use latex_ide_ui::button::ButtonVariant;

/// Desktop AI chat interface using shared chat engine
#[component]
pub fn DesktopAIChatInterface(
    #[cfg(feature = "latex-ide-model-manager")]
    model_manager: Option<Signal<Option<std::sync::Arc<ModelManager>>>>,
    #[cfg(not(feature = "latex-ide-model-manager"))]
    model_manager: Option<()>,
) -> Element {
    let mut chat_engine = use_signal(|| ChatEngine::new());
    let mut current_message = use_signal(|| String::new());
    
    rsx! {
        div { class: "h-full flex flex-col",
            
            // AI Chat header
            div { class: "p-4 border-b border-gray-200 dark:border-gray-700",
                h3 { class: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-2",
                    "AI Assistant"
                }
                
                // Model selection
                Dropdown {
                    items: chat_engine.read().available_models.iter().map(|model| {
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
            }
            
            // Chat messages
            div { class: "flex-1 overflow-auto p-4 space-y-4",
                if chat_engine.read().messages.is_empty() {
                    div { class: "text-center text-gray-500 mt-8",
                        "Start a conversation with your AI assistant.\nAsk about LaTeX syntax, document structure, or get help with your writing."
                    }
                } else {
                    for message in chat_engine.read().messages.iter() {
                        div { class: "bg-gray-100 dark:bg-gray-700 rounded-lg p-3",
                            "{message.content}"
                        }
                    }
                }
            }
            
            // Message input
            div { class: "p-4 border-t border-gray-200 dark:border-gray-700",
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
                                chat_engine.write().add_user_message(message);
                                chat_engine.write().add_assistant_message(
                                    "I can help you with that LaTeX question!".to_string()
                                );
                                current_message.set(String::new());
                            }
                        },
                        "Send"
                    }
                }
            }
        }
    }
}