use dioxus::prelude::*;
use latex_ide_ui::*;
use latex_ide_git_manager::{GitRepository, SessionManager, GitOperations, HistoryViewer, ConflictResolver, CommitInfo, GitStatus, BranchInfo};
use std::path::PathBuf;

pub mod components;
pub mod git_panel;
pub mod history_view;
pub mod branch_selector;
pub mod file_changes;
pub mod conflict_resolver;

pub use components::*;
pub use git_panel::GitPanel;
pub use history_view::HistoryView;
pub use branch_selector::BranchSelector;
pub use file_changes::FileChangesList;
pub use conflict_resolver::ConflictResolverUI;

#[derive(Clone, Debug)]
pub struct VersionControlState {
    pub git_repo: Option<GitRepository>,
    pub session_manager: Option<SessionManager>,
    pub git_operations: Option<GitOperations>,
    pub history_viewer: Option<HistoryViewer>,
    pub conflict_resolver: Option<ConflictResolver>,
    pub current_status: Option<GitStatus>,
    pub is_panel_visible: bool,
    pub selected_branch: Option<String>,
}

impl Default for VersionControlState {
    fn default() -> Self {
        Self {
            git_repo: None,
            session_manager: None,
            git_operations: None,
            history_viewer: None,
            conflict_resolver: None,
            current_status: None,
            is_panel_visible: false,
            selected_branch: None,
        }
    }
}

impl VersionControlState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self, repo_path: PathBuf) -> Result<(), anyhow::Error> {
        let git_repo = GitRepository::new(repo_path)?;
        let session_manager = SessionManager::new(git_repo.clone());
        let git_operations = GitOperations::new(git_repo.clone());
        let history_viewer = HistoryViewer::new(git_repo.clone());
        let conflict_resolver = ConflictResolver::new(git_repo.clone());

        self.git_repo = Some(git_repo);
        self.session_manager = Some(session_manager);
        self.git_operations = Some(git_operations);
        self.history_viewer = Some(history_viewer);
        self.conflict_resolver = Some(conflict_resolver);

        self.refresh_status()?;
        Ok(())
    }

    pub fn refresh_status(&mut self) -> Result<(), anyhow::Error> {
        if let Some(ref git_repo) = self.git_repo {
            self.current_status = Some(git_repo.get_status()?);
        }
        Ok(())
    }

    pub fn toggle_panel(&mut self) {
        self.is_panel_visible = !self.is_panel_visible;
    }

    pub fn set_panel_visible(&mut self, visible: bool) {
        self.is_panel_visible = visible;
    }

    pub fn is_initialized(&self) -> bool {
        self.git_repo.is_some()
    }

    pub fn has_repository(&self) -> bool {
        self.git_repo.as_ref().map_or(false, |repo| repo.is_repository())
    }

    pub fn get_current_branch(&self) -> Option<String> {
        self.current_status.as_ref().map(|status| status.current_branch.clone())
    }

    pub fn get_session_branch(&self) -> Option<String> {
        self.current_status.as_ref().and_then(|status| status.session_branch.clone())
    }

    pub fn has_changes(&self) -> bool {
        self.current_status.as_ref().map_or(false, |status| status.has_changes)
    }
}

#[component]
pub fn VersionControlProvider(children: Element) -> Element {
    let version_control_state = use_context_provider(|| Signal::new(VersionControlState::new()));

    rsx! {
        div { class: "version-control-provider",
            {children}
        }
    }
}

pub fn use_version_control() -> Signal<VersionControlState> {
    use_context::<Signal<VersionControlState>>()
}