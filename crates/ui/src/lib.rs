// Re-export common dioxus types for convenience
pub use dioxus::prelude::*;
pub use dioxus_signals::{GlobalSignal, Owner, Readable, Writable, Signal};
pub use dioxus_hooks::{use_signal, use_context, use_context_provider, use_resource};

pub mod button;
pub mod card;
pub mod dropdown;
pub mod input;
pub mod modal;
pub mod panel;
pub mod tabs;
pub mod theme;
pub mod icons;
pub mod layout;

// Platform-specific modules
pub mod web;
pub mod desktop;

pub use button::Button;
pub use card::Card;
pub use dropdown::{Dropdown, DropdownItem};
pub use input::Input;
pub use modal::Modal;
pub use panel::{ResizablePanel, PanelGroup};
pub use tabs::{Tabs, Tab};
pub use theme::{Theme, ThemeProvider, use_theme};
pub use icons::Icon;
pub use layout::{SplitView, Sidebar};