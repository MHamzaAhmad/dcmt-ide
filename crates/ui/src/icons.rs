use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum IconType {
    File,
    Folder,
    FolderOpen,
    Save,
    Copy,
    Paste,
    Cut,
    Undo,
    Redo,
    Search,
    Settings,
    Close,
    Minimize,
    Maximize,
    Play,
    Pause,
    Stop,
    Refresh,
    Download,
    Upload,
    User,
    Users,
    Message,
    Send,
    Menu,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
}

#[component]
pub fn Icon(
    icon: IconType,
    #[props(default = 20)] size: u32,
    #[props(default)] class: Option<String>,
) -> Element {
    let class_str = format!(
        "inline-block {}",
        class.unwrap_or_default()
    );
    
    let path_data = match icon {
        IconType::File => "M9 2a1 1 0 000 2h2a1 1 0 100-2H9z M4 5a2 2 0 012-2 1 1 0 000 2H6a2 2 0 00-2 2v6a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-1a1 1 0 100-2h1a4 4 0 014 4v6a4 4 0 01-4 4H6a4 4 0 01-4-4V7a4 4 0 014-4z",
        IconType::Folder => "M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z",
        IconType::FolderOpen => "M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z",
        IconType::Save => "M3 5a2 2 0 012-2h10a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V5zm2 0v10h10V5H5z",
        IconType::Settings => "M10 1.5a8.5 8.5 0 100 17 8.5 8.5 0 000-17zM10 4a6 6 0 110 12 6 6 0 010-12z",
        IconType::Close => "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z",
        IconType::Menu => "M3 5a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 10a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 15a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z",
        IconType::ChevronLeft => "M12.707 5.293a1 1 0 010 1.414L9.414 10l3.293 3.293a1 1 0 01-1.414 1.414l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 0z",
        IconType::ChevronRight => "M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z",
        _ => "M10 2a8 8 0 100 16 8 8 0 000-16z",
    };
    
    rsx! {
        svg {
            class: "{class_str}",
            width: "{size}",
            height: "{size}",
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 20 20",
            fill: "currentColor",
            path {
                fill_rule: "evenodd",
                d: "{path_data}",
                clip_rule: "evenodd"
            }
        }
    }
}