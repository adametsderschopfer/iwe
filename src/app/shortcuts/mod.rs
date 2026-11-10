//! Record settings shortcuts and route unhandled keyboard events by window.

use super::{App, Message, Navigation, SettingsTab, line_navigation};
use crate::{
    domain::shortcuts::{Command, Rejection},
    i18n::Key,
    ui::shortcuts,
};
use iced::{Task, event::Status, keyboard, window};

#[derive(Default)]
pub(crate) struct State {
    pub recording: Option<Command>,
    pub error: Option<Key>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Record(Command),
    Cancel,
    Key(window::Id, keyboard::Event, Status),
}

pub(crate) fn command_message(command: Command) -> Option<Message> {
    match command {
        Command::SaveFile => Some(Message::Editor(super::editor::Event::Save)),
        Command::GoToLine => Some(Message::GoToLine(line_navigation::Event::Open)),
        Command::CloseTab => Some(Message::CloseActiveTab),
        Command::ReopenTab => Some(Message::ReopenClosedTab),
        Command::Copy | Command::SelectAll => None,
        Command::DeploymentCompare => Some(super::modules::deployment_message(
            crate::modules::deployment::Event::CurrentFile(true),
        )),
        Command::DeploymentUpload => Some(super::modules::deployment_message(
            crate::modules::deployment::Event::CurrentFile(false),
        )),
    }
}

pub(super) fn key_message(
    key: &keyboard::Key,
    physical_key: keyboard::key::Physical,
    modifiers: keyboard::Modifiers,
    bindings: &crate::domain::shortcuts::Shortcuts,
) -> Option<Message> {
    if *key == keyboard::Key::Named(keyboard::key::Named::Escape) {
        return Some(Message::DismissDialog);
    }
    if modifiers == keyboard::Modifiers::COMMAND
        && (physical_key == keyboard::key::Physical::Code(keyboard::key::Code::Comma)
            || matches!(key, keyboard::Key::Character(value) if value == ","))
    {
        return Some(Message::Navigate(Navigation::Settings));
    }
    Command::ALL
        .into_iter()
        .find(|command| {
            shortcuts::matches(&bindings.binding(*command), key, physical_key, modifiers)
        })
        .and_then(command_message)
}

impl App {
    pub(super) fn update_shortcuts(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Record(command) => {
                self.shortcuts.recording = Some(command);
                self.shortcuts.error = None;
            }
            Event::Cancel => self.shortcuts = State::default(),
            Event::Key(
                id,
                keyboard::Event::KeyPressed {
                    key,
                    physical_key,
                    modifiers,
                    repeat: false,
                    ..
                },
                status,
            ) => {
                let settings = self.windows.settings == Some(id)
                    || (self.window_id == Some(id) && self.navigation == Navigation::Settings);
                if settings
                    && self.settings_tab == SettingsTab::Shortcuts
                    && let Some(command) = self.shortcuts.recording
                {
                    if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                        self.shortcuts = State::default();
                        return Task::none();
                    }
                    if shortcuts::modifier(&key) {
                        return Task::none();
                    }
                    let result = shortcuts::binding(&key, physical_key, modifiers)
                        .ok_or(Rejection::Invalid)
                        .and_then(|binding| {
                            self.workspace
                                .preferences
                                .shortcuts
                                .assign(command, binding)
                        });
                    match result {
                        Ok(()) => {
                            self.shortcuts = State::default();
                            return self.save();
                        }
                        Err(reason) => {
                            self.shortcuts.error = Some(match reason {
                                Rejection::Invalid => Key::ShortcutInvalid,
                                Rejection::Conflict => Key::ShortcutConflict,
                            })
                        }
                    }
                    return Task::none();
                }
                if status == Status::Captured {
                    return Task::none();
                }
                let message = key_message(
                    &key,
                    physical_key,
                    modifiers,
                    &self.workspace.preferences.shortcuts,
                );
                if let Some(message) = message {
                    return self.update(Message::WindowShortcut(id, Box::new(message)));
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
