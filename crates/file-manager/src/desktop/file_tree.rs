use dioxus::prelude::*;
use dioxus_signals::Readable;
use dioxus_hooks::use_signal;
use crate::ProjectManager;
use latex_ide_ui::*;

/// Desktop file tree component
#[component]
pub fn DesktopFileTree(onfile_select: EventHandler<String>) -> Element {
    let _project_manager = use_signal(|| ProjectManager::new());
    
    rsx! {
        div { class: "h-full overflow-auto p-2",
            div { class: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                "Files"
            }
            
            // Mock file tree for now - same as original desktop implementation
            div { class: "space-y-1",
                FileItem { 
                    name: "document.tex",
                    is_file: true,
                    onclick: move |_| onfile_select.call("document.tex".to_string())
                }
                FileItem { 
                    name: "figures/",
                    is_file: false,
                    onclick: move |_| {}
                }
                FileItem { 
                    name: "references.bib",
                    is_file: true,
                    onclick: move |_| onfile_select.call("references.bib".to_string())
                }
            }
        }
    }
}

/// Desktop file item component
#[component]
fn FileItem(name: String, is_file: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let icon = if is_file { "📄" } else { "📁" };
    
    rsx! {
        div { 
            class: "flex items-center space-x-2 px-2 py-1 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer",
            onclick: move |evt| onclick.call(evt),
            
            span { "{icon}" }
            span { "{name}" }
        }
    }
}