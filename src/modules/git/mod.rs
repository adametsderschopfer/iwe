//! Bundled git state and public integration boundary.

use crate::domain::{
    Failure,
    repository::{
        ChangedFile,
        project::{BranchKey, CommitKey},
    },
};
use actions as git_actions;
use iced::widget::pane_grid;
use selection as branch_selection;
use std::collections::BTreeSet;
pub(crate) mod actions;
pub(crate) mod selection;
pub(crate) mod transport;

#[derive(Debug, Clone, Copy)]
pub(crate) enum GitPanel {
    Branches,
    Commits,
    Files,
}

pub(crate) fn git_panes() -> pane_grid::State<GitPanel> {
    let (mut panes, branches) = pane_grid::State::new(GitPanel::Branches);
    let (_, left) = panes
        .split(pane_grid::Axis::Vertical, branches, GitPanel::Commits)
        .unwrap();
    panes.resize(left, 0.22);
    panes
}

pub(crate) struct State {
    pub git_panes: pane_grid::State<GitPanel>,
    pub history_loading: bool,
    pub history_error: Option<Failure>,
    pub history_scroll_y: f32,
    pub git_actions: git_actions::State,
    pub branch: Option<BranchKey>,
    pub branch_selection: branch_selection::State,
    pub branch_attention: Option<u64>,
    pub branch_attention_request: u64,
    pub query: String,
    pub selected_commit: Option<CommitKey>,
    pub changed_files: Vec<ChangedFile>,
    pub collapsed_commit_folders: BTreeSet<String>,
    pub expanded_branch_folders: BTreeSet<String>,
    pub changes_loading: bool,
    pub changes_error: Option<Failure>,
    pub copied_commit: Option<String>,
    pub copy_request: u64,
    pub commit_request: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            git_panes: git_panes(),
            history_loading: false,
            history_error: None,
            history_scroll_y: 0.,
            git_actions: git_actions::State::default(),
            branch: None,
            branch_selection: branch_selection::State::default(),
            branch_attention: None,
            branch_attention_request: 0,
            query: String::new(),
            selected_commit: None,
            changed_files: Vec::new(),
            collapsed_commit_folders: BTreeSet::new(),
            expanded_branch_folders: BTreeSet::from(["false".to_owned()]),
            changes_loading: false,
            changes_error: None,
            copied_commit: None,
            copy_request: 0,
            commit_request: 0,
        }
    }
}

mod view;
pub(crate) use view::{Event, View, activity, overlay, view};
