//! Reveal numbered source lines without loading a large file into memory.

use super::{App, Message, Navigation};
use crate::{
    domain::{Failure, changes::TextCursor, workspace::DiffView},
    i18n::Key,
    ui::code::{
        Document, LineChange,
        folding::{Row, SideRow},
        selection::{Action, Lane},
    },
};
use iced::{Task, widget::operation};

#[cfg(test)]
mod tests;

pub(crate) struct Dialog {
    pub id: u64,
    pub request: u64,
    pub value: String,
    pub error: Option<Key>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedPage {
    pub document: Box<Document>,
    pub history: Vec<TextCursor>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open,
    Changed(String),
    Submit,
    Cancel,
    Loaded(u64, u64, u64, Result<Option<LoadedPage>, Failure>),
}

impl App {
    pub(super) fn update_line_navigation(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open if self.line_navigation.is_some() => {
                return operation::focus("go-to-line-number");
            }
            Event::Open
                if self.navigation != Navigation::Settings
                    && self.space_draft.is_none()
                    && self.project_add.is_none()
                    && self.auth_dialog.is_none()
                    && !self.git_actions.has_dialog()
                    && self.preview().document.is_some()
                    && !self.preview().loading =>
            {
                self.files.menu = None;
                self.project_menu = None;
                self.repository_menu = None;
                self.space_menu = None;
                self.git_actions.menu = None;
                self.git_actions.bulk.menu = None;
                self.file_request += 1;
                self.line_navigation = Some(Dialog {
                    id: self.preview().id,
                    request: self.file_request,
                    value: String::new(),
                    error: None,
                    loading: false,
                });
                return operation::focus("go-to-line-number");
            }
            Event::Changed(value) => {
                if let Some(dialog) = self
                    .line_navigation
                    .as_mut()
                    .filter(|dialog| !dialog.loading)
                {
                    dialog.value = value.chars().take(20).collect();
                    dialog.error = None;
                }
            }
            Event::Cancel => self.line_navigation = None,
            Event::Submit => {
                let Some(dialog) = self
                    .line_navigation
                    .as_ref()
                    .filter(|dialog| !dialog.loading)
                else {
                    return Task::none();
                };
                let Some(line) = dialog
                    .value
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|line| *line > 0)
                else {
                    if let Some(dialog) = &mut self.line_navigation {
                        dialog.error = Some(Key::InvalidLineNumber);
                    }
                    return Task::none();
                };
                if dialog.id != self.preview().id {
                    self.line_navigation = None;
                    return Task::none();
                }
                if self.reveal_source_line(line) {
                    self.line_navigation = None;
                    return self.restore_preview_scroll();
                }
                if self
                    .preview()
                    .document
                    .as_ref()
                    .is_some_and(|document| document.page.is_some())
                {
                    let id = self.preview().id;
                    let Some(dialog) = &mut self.line_navigation else {
                        return Task::none();
                    };
                    let request = dialog.request;
                    dialog.loading = true;
                    return self.load_line_preview(id, request, line);
                }
                if let Some(dialog) = &mut self.line_navigation {
                    dialog.error = Some(Key::LineNumberNotFound);
                }
            }
            Event::Loaded(generation, id, request, result)
                if generation == self.generation
                    && self.preview().id == id
                    && self.line_navigation.as_ref().is_some_and(|dialog| {
                        dialog.id == id && dialog.request == request && dialog.loading
                    }) =>
            {
                match result {
                    Ok(Some(page)) => {
                        let line = self
                            .line_navigation
                            .as_ref()
                            .and_then(|dialog| dialog.value.trim().parse().ok())
                            .unwrap_or(1);
                        let preparation = self.accept_line_preview(*page.document, page.history);
                        self.reveal_source_line(line);
                        self.line_navigation = None;
                        return Task::batch([preparation, self.restore_preview_scroll()]);
                    }
                    result => {
                        if let Some(dialog) = &mut self.line_navigation {
                            dialog.loading = false;
                            dialog.error = Some(match result {
                                Ok(None) => Key::LineNumberNotFound,
                                Err(error) => super::error_key(error),
                                _ => Key::OpenFailed,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn reveal_source_line(&mut self, line: usize) -> bool {
        let side_by_side = self.workspace.preferences.diff_view == DiffView::SideBySide;
        let size = self.workspace.preferences.code_size as f32;
        let id = self.preview().id;
        let Some(preview) = self.preview_for_request(id) else {
            return false;
        };
        let Some(document) = preview.document.clone() else {
            return false;
        };
        let lane = if side_by_side && document.comparison.is_some() {
            let fallback =
                if document.comparison.as_ref().is_some_and(|comparison| {
                    comparison.rows.iter().any(|row| row.cells[1].is_some())
                }) {
                    Lane::After
                } else {
                    Lane::Before
                };
            preview
                .selection
                .lane()
                .filter(|lane| *lane != Lane::Unified)
                .unwrap_or(fallback)
        } else {
            Lane::Unified
        };
        let side = usize::from(lane == Lane::After);
        let index = if lane != Lane::Unified {
            document.comparison.as_ref().and_then(|comparison| {
                comparison
                    .rows
                    .iter()
                    .filter_map(|row| row.cells[side].as_ref())
                    .find(|cell| cell.number == line)
                    .map(|cell| cell.index)
            })
        } else {
            document
                .line_info
                .iter()
                .enumerate()
                .find(|(_, info)| info.number == line && info.change != Some(LineChange::Removed))
                .or_else(|| {
                    document
                        .line_info
                        .iter()
                        .enumerate()
                        .find(|(_, info)| info.number == line)
                })
                .map(|(index, _)| index)
        };
        let Some(index) = index else {
            return false;
        };
        let hidden =
            (0..preview.folding.len(&document)).find_map(|row| match preview.folding.row(row) {
                Row::Fold(range) if range.contains(&index) => Some(range),
                _ => None,
            });
        if let Some(range) = hidden {
            preview.folding.expand(&document, range);
        }
        let (row, coordinate) = if lane == Lane::Unified {
            let row = (0..preview.folding.len(&document))
                .find(|row| preview.folding.row(*row) == Row::Line(index));
            (row, index)
        } else {
            let Some(comparison) = document.comparison.as_ref() else {
                return false;
            };
            let pair = comparison.rows.iter().position(|row| {
                row.cells[side]
                    .as_ref()
                    .is_some_and(|cell| cell.index == index)
            });
            let row = pair.and_then(|pair| {
                (0..preview.folding.side_len(comparison)).find(|row|
                matches!(preview.folding.side_row(*row), SideRow::Line(index) if index == pair))
            });
            (row, line - 1)
        };
        let Some(row) = row else {
            return false;
        };
        let y = row.saturating_sub(3) as f32 * size * 1.65;
        if lane == Lane::Unified {
            preview.scroll_x = 0.;
            preview.scroll_y = y;
        } else {
            preview.diff_scroll_x = [0.; 2];
            preview.diff_scroll_y = y;
        }
        let length = document.lines[index]
            .iter()
            .map(|(text, _)| text.len())
            .sum();
        preview
            .selection
            .update(Action::Line(lane, coordinate, length), &document);
        true
    }
}
