//! Coordinate text actions, asynchronous saves, and unsaved-change decisions.

use super::{App, Message};
use crate::{domain::editor::Document, i18n::Key, infrastructure::editor};
use iced::{Task, widget::text_editor};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Local(PathBuf),
    Remote {
        profile: String,
        label: String,
        path: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct Buffer {
    pub document: Document,
    pub content: text_editor::Content,
    pub target: Target,
    pub reviewing: bool,
    pub saving: bool,
    pub error: Option<Key>,
    pub saved_notice: bool,
    token: u64,
    snapshot: String,
}

impl Buffer {
    pub(crate) fn new(text: String, target: Target) -> Self {
        Self {
            content: text_editor::Content::with_text(&text),
            document: Document::new(text),
            target,
            reviewing: false,
            saving: false,
            error: None,
            saved_notice: false,
            token: 0,
            snapshot: String::new(),
        }
    }
    pub(crate) fn protected(&self) -> bool {
        self.saving || self.document.dirty()
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub pending: Option<Box<Message>>,
    pub saving_all: bool,
    next_token: u64,
    bypass: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Action(u64, text_editor::Action),
    Undo(u64, bool),
    Save,
    Saved(u64, Result<(), Key>),
    ToggleReview(u64),
    SaveAll,
    Discard,
    Cancel,
}

impl App {
    pub(crate) fn editor_buffer(&self) -> Option<&Buffer> {
        self.preview().editor.as_ref()
    }

    pub(crate) fn editor_pending_tab(&self, id: u64) -> bool {
        match self.editor.pending.as_deref() {
            Some(Message::CloseTab(target)) => id == *target,
            Some(Message::CloseActiveTab) => id == self.preview().id,
            _ => true,
        }
    }

    pub(super) fn protect_editor(&mut self, message: &Message) -> bool {
        if self.editor.bypass {
            self.editor.bypass = false;
            return false;
        }
        let target = match message {
            Message::CloseTab(id) => Some(*id),
            Message::CloseActiveTab => Some(self.preview().id),
            Message::SelectProject(..)
            | Message::RemoveProjectFromSpace
            | Message::Space(
                super::spaces::Message::Delete(_)
                | super::spaces::Message::Create
                | super::spaces::Message::RemoveProject(_)
                | super::spaces::Message::ProjectsPicked(..),
            )
            | Message::Update(super::updates::Event::Install)
            | Message::Module(super::modules::Event::SetEnabled(_, false)) => None,
            Message::Window(super::windows::Event::Close(id)) if self.window_id == Some(*id) => {
                None
            }
            Message::Module(super::modules::Event::Deployment(
                crate::modules::deployment::Event::SpaceSettings(_),
            )) => None,
            _ => return false,
        };
        if self.tabs.files.iter().any(|tab| {
            target.is_none_or(|id| tab.id == id)
                && tab.editor.as_ref().is_some_and(Buffer::protected)
        }) {
            if self.editor.pending.is_none() {
                self.editor.pending = Some(Box::new(message.clone()));
            }
            return true;
        }
        false
    }

    fn save_editor(&mut self, id: u64) -> Task<Message> {
        if self.editor_buffer().is_none() && !self.editor.saving_all {
            return Task::none();
        }
        self.editor.next_token += 1;
        let token = self.editor.next_token;
        let Some(buffer) = self
            .tabs
            .files
            .iter_mut()
            .find(|tab| tab.id == id)
            .and_then(|tab| tab.editor.as_mut())
        else {
            return Task::none();
        };
        if buffer.saving || !buffer.document.dirty() {
            return Task::none();
        }
        buffer.token = token;
        buffer.snapshot = buffer.document.text.clone();
        buffer.saving = true;
        buffer.error = None;
        buffer.saved_notice = false;
        let text = buffer.snapshot.clone();
        let expected = buffer.document.saved.clone();
        match buffer.target.clone() {
            Target::Local(path) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || editor::save(&path, &expected, &text))
                        .await
                        .unwrap_or(Err(editor::Error::Access))
                        .map_err(|error| match error {
                            editor::Error::Changed => Key::EditorConflict,
                            editor::Error::TooLarge => Key::EditorTooLarge,
                            editor::Error::Symlink => Key::EditorSymlink,
                            editor::Error::Access => Key::EditorSaveFailed,
                        })
                },
                move |result| Message::Editor(Event::Saved(token, result)),
            ),
            Target::Remote { profile, path, .. } => {
                if self.modules.deployment.busy {
                    return self.update_editor(Event::Saved(token, Err(Key::EditorBusy)));
                }
                if !self.module_enabled(crate::modules::ModuleId::Deployment) {
                    return self
                        .update_editor(Event::Saved(token, Err(Key::EditorDeploymentDisabled)));
                }
                self.update_module(super::modules::Event::Deployment(
                    crate::modules::deployment::Event::SaveText {
                        profile,
                        token,
                        path,
                        expected: Arc::new(expected),
                        text: Arc::new(text),
                    },
                ))
            }
        }
    }

    fn continue_editor_action(&mut self) -> Task<Message> {
        if self.tabs.files.iter().any(|tab| {
            self.editor_pending_tab(tab.id) && tab.editor.as_ref().is_some_and(Buffer::protected)
        }) {
            return Task::none();
        }
        self.editor.saving_all = false;
        if let Some(message) = self.editor.pending.take() {
            self.editor.bypass = true;
            return self.update(*message);
        }
        Task::none()
    }

    pub(super) fn update_editor(&mut self, event: Event) -> Task<Message> {
        if !matches!(
            event,
            Event::Saved(..) | Event::SaveAll | Event::Discard | Event::Cancel
        ) && (self.auth_dialog.is_some()
            || self.line_navigation.is_some()
            || self.space_draft.is_some()
            || self.project_add.is_some()
            || self.modules.git.git_actions.has_dialog()
            || self.modules.deployment.has_dialog()
            || self.navigation == super::Navigation::Settings)
        {
            return Task::none();
        }
        match event {
            Event::Action(id, action) => {
                if self.editor.pending.is_some()
                    || self.auth_dialog.is_some()
                    || self.line_navigation.is_some()
                    || self.space_draft.is_some()
                    || self.project_add.is_some()
                    || self.modules.git.git_actions.has_dialog()
                {
                    return Task::none();
                }
                if let Some(buffer) = self
                    .tabs
                    .files
                    .iter_mut()
                    .find(|tab| tab.id == id)
                    .and_then(|tab| tab.editor.as_mut())
                {
                    let edited = action.is_edit();
                    buffer.content.perform(action);
                    if edited {
                        let text = buffer.content.text();
                        if text.len() > crate::domain::editor::LIMIT {
                            buffer.content = text_editor::Content::with_text(&buffer.document.text);
                            buffer.error = Some(Key::EditorTooLarge);
                        } else {
                            buffer.document.edit(text);
                            buffer.saved_notice = false;
                        }
                    }
                }
            }
            Event::Undo(id, redo) if self.editor.pending.is_none() => {
                if let Some(buffer) = self
                    .tabs
                    .files
                    .iter_mut()
                    .find(|tab| tab.id == id)
                    .and_then(|tab| tab.editor.as_mut())
                {
                    let cursor = buffer.content.cursor();
                    buffer.document.undo(redo);
                    buffer.content = text_editor::Content::with_text(&buffer.document.text);
                    buffer.content.move_to(cursor);
                    buffer.saved_notice = false;
                }
            }
            Event::ToggleReview(id) => {
                if let Some(buffer) = self
                    .tabs
                    .files
                    .iter_mut()
                    .find(|tab| tab.id == id)
                    .and_then(|tab| tab.editor.as_mut())
                {
                    buffer.reviewing = !buffer.reviewing;
                }
            }
            Event::Save if self.editor.pending.is_none() => {
                return self.save_editor(self.preview().id);
            }
            Event::Saved(token, result) => {
                let Some(tab) = self.tabs.files.iter_mut().find(|tab| {
                    tab.editor
                        .as_ref()
                        .is_some_and(|buffer| buffer.token == token && token != 0)
                }) else {
                    return Task::none();
                };
                let buffer = tab.editor.as_mut().expect("Matched editor");
                buffer.saving = false;
                buffer.error = result.err();
                buffer.saved_notice = result.is_ok();
                if result.is_ok() {
                    buffer.document.saved.clone_from(&buffer.snapshot);
                } else {
                    self.editor.saving_all = false;
                }
                let id = tab.id;
                let refresh = if result.is_ok() {
                    self.refresh_saved_editor(id)
                } else {
                    Task::none()
                };
                if self.editor.saving_all {
                    return Task::batch([refresh, self.save_next_editor()]);
                }
                return refresh;
            }
            Event::SaveAll => {
                self.editor.saving_all = true;
                return self.save_next_editor();
            }
            Event::Discard => {
                if self
                    .tabs
                    .files
                    .iter()
                    .any(|tab| tab.editor.as_ref().is_some_and(|buffer| buffer.saving))
                {
                    return Task::none();
                }
                let target = self
                    .editor
                    .pending
                    .as_deref()
                    .and_then(|message| match message {
                        Message::CloseTab(id) => Some(*id),
                        Message::CloseActiveTab => Some(self.preview().id),
                        _ => None,
                    });
                for tab in &mut self.tabs.files {
                    if target.is_none_or(|id| tab.id == id)
                        && let Some(buffer) = &mut tab.editor
                    {
                        self.modules.deployment.forget_editor_save(buffer.token);
                        *buffer = Buffer::new(buffer.document.saved.clone(), buffer.target.clone());
                    }
                }
                self.editor.saving_all = false;
                if let Some(message) = self.editor.pending.take() {
                    self.editor.bypass = true;
                    return self.update(*message);
                }
            }
            Event::Cancel => {
                self.editor.pending = None;
                self.editor.saving_all = false;
            }
            _ => {}
        }
        Task::none()
    }

    fn save_next_editor(&mut self) -> Task<Message> {
        if self
            .tabs
            .files
            .iter()
            .any(|tab| tab.editor.as_ref().is_some_and(|buffer| buffer.saving))
        {
            return Task::none();
        }
        if let Some(id) = self
            .tabs
            .files
            .iter()
            .find(|tab| {
                self.editor_pending_tab(tab.id)
                    && tab
                        .editor
                        .as_ref()
                        .is_some_and(|buffer| buffer.document.dirty())
            })
            .map(|tab| tab.id)
        {
            self.save_editor(id)
        } else {
            self.continue_editor_action()
        }
    }
}

#[cfg(test)]
mod tests;
