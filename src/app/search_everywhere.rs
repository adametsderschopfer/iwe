//! Search project files and switch projects from one keyboard-driven popup.

use super::{App, Message, Navigation};
use crate::{
    domain::{search, search::FileCandidate, workspace::Workspace},
    ui::theme,
};
use iced::{Task, widget::operation};
use std::path::PathBuf;

#[cfg(test)]
mod tests;

pub(crate) const INPUT_ID: &str = "search-everywhere-input";
pub(crate) const RESULTS_ID: &str = "search-everywhere-results";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Scope {
    #[default]
    All,
    Files,
    Projects,
}

impl Scope {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Files,
            Self::Files => Self::Projects,
            Self::Projects => Self::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Row {
    File {
        name: String,
        directory: String,
        path: PathBuf,
    },
    Project {
        space: usize,
        project: usize,
    },
}

pub(crate) struct Dialog {
    pub scope: Scope,
    pub query: String,
    pub rows: Vec<Row>,
    pub selection: usize,
}

#[derive(Default)]
pub(crate) struct State {
    pub dialog: Option<Dialog>,
    files: Vec<FileCandidate>,
    dirty: bool,
}

impl State {
    /// Rebuild the cached file candidates on the next query change.
    pub(crate) fn tree_changed(&mut self) {
        self.dirty = true;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Open(Scope),
    ChangeScope(Scope),
    Changed(String),
    Moved(bool),
    CycleScope,
    Activate(usize),
    Selected,
    Cancel,
}

impl App {
    pub(super) fn update_search_everywhere(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(_) if self.search_everywhere.dialog.is_some() => {
                return operation::focus(INPUT_ID);
            }
            Event::Open(scope)
                if self.navigation != Navigation::Settings
                    && self.workspace.directory().is_some()
                    && self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && self.line_navigation.is_none()
                    && self.editor.pending.is_none()
                    && !self.modules.git.git_actions.has_dialog() =>
            {
                self.files.menu = None;
                self.project_menu = None;
                self.repository_menu = None;
                self.space_menu = None;
                self.modules.git.git_actions.menu = None;
                self.modules.git.git_actions.bulk.menu = None;
                let files = self.collect_file_candidates();
                let state = &mut self.search_everywhere;
                state.files = files;
                state.dirty = false;
                state.dialog = Some(Dialog {
                    scope,
                    query: String::new(),
                    rows: Vec::new(),
                    selection: 0,
                });
                return operation::focus(INPUT_ID);
            }
            Event::ChangeScope(scope) => {
                if let Some(dialog) = self.search_everywhere.dialog.as_mut()
                    && dialog.scope != scope
                {
                    dialog.scope = scope;
                    return self.refresh_search_rows();
                }
            }
            Event::CycleScope => {
                if let Some(dialog) = self.search_everywhere.dialog.as_mut() {
                    dialog.scope = dialog.scope.next();
                    return self.refresh_search_rows();
                }
            }
            Event::Changed(query) => {
                if self.search_everywhere.dialog.is_some() && self.search_everywhere.dirty {
                    self.search_everywhere.files = self.collect_file_candidates();
                    self.search_everywhere.dirty = false;
                }
                if let Some(dialog) = self.search_everywhere.dialog.as_mut() {
                    dialog.query = query.chars().take(200).collect();
                }
                return self.refresh_search_rows();
            }
            Event::Moved(forward) => {
                let mut scroll = Task::none();
                if let Some(dialog) = self.search_everywhere.dialog.as_mut()
                    && !dialog.rows.is_empty()
                {
                    let length = dialog.rows.len();
                    dialog.selection = if forward {
                        (dialog.selection + 1) % length
                    } else {
                        (dialog.selection + length - 1) % length
                    };
                    scroll = selection_scroll(&dialog.rows, dialog.selection);
                }
                return scroll;
            }
            Event::Activate(index) => {
                if let Some(dialog) = self.search_everywhere.dialog.as_mut()
                    && index < dialog.rows.len()
                {
                    dialog.selection = index;
                    return self.activate_search_row();
                }
            }
            Event::Selected => return self.activate_search_row(),
            Event::Cancel => self.search_everywhere.dialog = None,
            _ => {}
        }
        Task::none()
    }

    fn activate_search_row(&mut self) -> Task<Message> {
        let Some(dialog) = self.search_everywhere.dialog.take() else {
            return Task::none();
        };
        match dialog.rows.into_iter().nth(dialog.selection) {
            Some(Row::File { path, .. }) => self.open_file(path),
            Some(Row::Project { space, project }) => {
                self.update(Message::SelectProject(space, project))
            }
            None => Task::none(),
        }
    }

    fn refresh_search_rows(&mut self) -> Task<Message> {
        let state = &mut self.search_everywhere;
        let Some(dialog) = state.dialog.as_mut() else {
            return Task::none();
        };
        dialog.rows = collect_rows(dialog.scope, &dialog.query, &state.files, &self.workspace);
        dialog.selection = 0;
        operation::scroll_to(
            iced::widget::Id::new(RESULTS_ID),
            operation::AbsoluteOffset { x: 0., y: 0. },
        )
    }

    fn collect_file_candidates(&self) -> Vec<FileCandidate> {
        let Some(root) = self.workspace.directory() else {
            return Vec::new();
        };
        self.tree
            .values()
            .flatten()
            .filter(|entry| !entry.directory)
            .map(|entry| FileCandidate {
                directory: entry
                    .path
                    .strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.parent())
                    .map(|parent| {
                        parent
                            .iter()
                            .map(|component| component.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default(),
                name: entry.name.clone(),
                path: entry.path.clone(),
            })
            .collect()
    }
}

/// Vertical scroll offset that keeps the selected row centered in the results list.
///
/// The offset mirrors the rendered column: a fixed-height group header precedes each
/// group, and rows follow at a fixed pitch.
fn selection_offset(rows: &[Row], selection: usize) -> Option<f32> {
    if selection >= rows.len() {
        return None;
    }
    let pitch = theme::SEARCH_ROW_HEIGHT + theme::SEARCH_ROW_SPACING;
    let mut offset = 0.;
    let mut group: Option<bool> = None;
    for (index, row) in rows.iter().enumerate() {
        let is_file = matches!(row, Row::File { .. });
        if group != Some(is_file) {
            group = Some(is_file);
            offset += theme::SEARCH_GROUP_HEIGHT + theme::SEARCH_ROW_SPACING;
        }
        if index == selection {
            break;
        }
        offset += pitch;
    }
    Some((offset + theme::SEARCH_ROW_HEIGHT / 2. - theme::SEARCH_RESULTS_HEIGHT / 2.).max(0.))
}

fn selection_scroll(rows: &[Row], selection: usize) -> Task<Message> {
    let offset = selection_offset(rows, selection).unwrap_or(0.);
    operation::scroll_to(
        iced::widget::Id::new(RESULTS_ID),
        operation::AbsoluteOffset { x: 0., y: offset },
    )
}

fn collect_rows(
    scope: Scope,
    query: &str,
    files: &[FileCandidate],
    workspace: &Workspace,
) -> Vec<Row> {
    let mut rows = Vec::new();
    if query.is_empty() {
        return rows;
    }
    if scope != Scope::Projects {
        rows.extend(search::rank(query, files).into_iter().map(|index| {
            let file = &files[index];
            Row::File {
                name: file.name.clone(),
                directory: file.directory.clone(),
                path: file.path.clone(),
            }
        }));
    }
    if scope != Scope::Files {
        let mut projects = Vec::new();
        for (space, space_value) in workspace.spaces.iter().enumerate() {
            for (project, project_value) in space_value.projects.iter().enumerate() {
                if let Some(score) = search::score(query, &project_value.name) {
                    projects.push((score, space, project));
                }
            }
        }
        projects.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        rows.extend(
            projects
                .into_iter()
                .map(|(_, space, project)| Row::Project { space, project }),
        );
    }
    rows
}
