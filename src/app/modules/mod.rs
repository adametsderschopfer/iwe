//! Compose enabled module runtimes and route their typed events.

use crate::modules::terminal;

use super::{App, Message};
use crate::modules::ModuleId;
use iced::{Subscription, Task};
mod deployment;

#[derive(Default)]
pub(crate) struct State {
    pub terminal: terminal::State,
    pub deployment: crate::modules::deployment::State,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Toggle(ModuleId),
    Terminal(terminal::Event),
    Deployment(crate::modules::deployment::Event),
}

impl App {
    pub(super) fn update_module(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Toggle(id) if self.workspace.directory().is_some() => {
                match id {
                    ModuleId::Deployment => {
                        return self.update_deployment(crate::modules::deployment::Event::Toggle);
                    }
                    ModuleId::Terminal => {
                        self.modules.terminal.open = !self.modules.terminal.open;
                        self.sync_panels();
                        if self.modules.terminal.open && self.modules.terminal.tabs.is_empty() {
                            return self.update_terminal(terminal::Event::Start);
                        }
                    }
                }
                Task::none()
            }
            Event::Terminal(event) => self.update_terminal(event),
            Event::Deployment(event) => self.update_deployment(event),
            Event::Toggle(_) => Task::none(),
        }
    }

    pub(super) fn module_subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.modules.terminal.subscription().map(terminal_message),
            self.modules
                .deployment
                .subscription()
                .map(deployment_message),
        ])
    }

    fn update_terminal(&mut self, event: terminal::Event) -> Task<Message> {
        match event {
            terminal::Event::Focus(request) => {
                if self.modules.terminal.active == Some(request) {
                    self.branch_selection.selected.clear();
                    for preview in &mut self.tabs.files {
                        if let Some(document) = &preview.document {
                            preview
                                .selection
                                .update(crate::ui::code::selection::Action::Clear, document);
                        }
                    }
                }
                return Task::none();
            }
            terminal::Event::PasteRejected => {
                self.notifications
                    .error(crate::i18n::Key::TerminalPasteTooLarge, None);
                return Task::none();
            }
            _ => {}
        }
        let panel_changed = matches!(
            event,
            terminal::Event::Start | terminal::Event::Hide | terminal::Event::Close(_)
        );
        let task = self
            .modules
            .terminal
            .update(
                event,
                self.workspace.directory().map(std::path::PathBuf::as_path),
            )
            .map(terminal_message);
        if panel_changed {
            self.sync_panels();
        }
        task
    }

    pub(crate) fn module_open(&self, id: ModuleId) -> bool {
        match id {
            ModuleId::Terminal => self.modules.terminal.open,
            ModuleId::Deployment => self.modules.deployment.open,
        }
    }
}

fn terminal_message(event: terminal::Event) -> Message {
    Message::Module(Event::Terminal(event))
}

pub(crate) fn deployment_message(event: crate::modules::deployment::Event) -> Message {
    Message::Module(Event::Deployment(event))
}

#[cfg(test)]
mod tests;
