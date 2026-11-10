//! Project localized workspace entries into the native application menu.

use super::{App, Message, Navigation, SettingsTab, spaces, windows};
use crate::{
    i18n::Key,
    infrastructure::native_menu::{self, Action, Model},
};
use iced::{Subscription, Task, window};
use std::time::Duration;

#[derive(Default)]
pub(super) struct State {
    pending: bool,
    unavailable: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Poll,
    Polled(Result<Vec<Action>, ()>),
}

impl App {
    pub(super) fn native_menu_subscription(&self) -> Subscription<Message> {
        if self.window_id.is_some() && !self.native_menu.unavailable {
            iced::time::every(Duration::from_millis(100)).map(|_| Message::NativeMenu(Event::Poll))
        } else {
            Subscription::none()
        }
    }

    fn menu_model(&self) -> Model {
        Model {
            application: self.i18n.text(Key::AppTitle),
            settings: self.i18n.text(Key::Settings),
            about: self.i18n.text(Key::About),
            spaces_title: self.i18n.text(Key::Spaces),
            projects_title: self.i18n.text(Key::Projects),
            new_space: self.i18n.text(Key::NewSpace),
            hide: self.i18n.text(Key::MenuHide),
            hide_others: self.i18n.text(Key::MenuHideOthers),
            show_all: self.i18n.text(Key::MenuShowAll),
            services: self.i18n.text(Key::MenuServices),
            quit: self.i18n.text(Key::MenuQuit),
            enabled: self.loaded
                && self.space_draft.is_none()
                && self.auth_dialog.is_none()
                && !self.git_actions.has_dialog()
                && self.line_navigation.is_none(),
            spaces: self
                .workspace
                .spaces
                .iter()
                .map(|space| {
                    (
                        space.name.clone(),
                        space
                            .projects
                            .iter()
                            .map(|project| (project.name.clone(), project.directory.clone()))
                            .collect(),
                    )
                })
                .collect(),
        }
    }

    pub(super) fn update_native_menu(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Poll if !self.native_menu.pending && !self.native_menu.unavailable => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                let model = self.menu_model();
                self.native_menu.pending = true;
                return window::run(id, move |_| native_menu::sync_and_poll(model))
                    .map(|result| Message::NativeMenu(Event::Polled(result)));
            }
            Event::Polled(result) => {
                self.native_menu.pending = false;
                let Ok(actions) = result else {
                    self.native_menu.unavailable = true;
                    self.notifications.error(Key::MenuUnavailable, None);
                    return Task::none();
                };
                let mut tasks = Vec::new();
                for action in actions {
                    let task = match action {
                        Action::Settings => self.update(Message::Navigate(Navigation::Settings)),
                        Action::About => {
                            self.settings_tab = SettingsTab::About;
                            self.update(Message::Navigate(Navigation::Settings))
                        }
                        Action::Quit => self.window_id.map_or_else(Task::none, |id| {
                            self.update_window(windows::Event::Close(id))
                        }),
                        action if self.menu_model().enabled => {
                            if let Some(id) = self.window_id {
                                tasks.push(
                                    window::minimize::<Message>(id, false)
                                        .chain(window::gain_focus(id)),
                                );
                            }
                            match action {
                                Action::NewSpace => {
                                    self.update(Message::Space(spaces::Message::New))
                                }
                                Action::Project(space, path) => {
                                    let project =
                                        self.workspace.spaces.get(space).and_then(|space| {
                                            space
                                                .projects
                                                .iter()
                                                .position(|project| project.directory == path)
                                        });
                                    project.map_or_else(Task::none, |project| {
                                        self.update(Message::SelectProject(space, project))
                                    })
                                }
                                Action::Space(space) if space < self.workspace.spaces.len() => {
                                    self.collapsed_spaces.remove(&space);
                                    if self.workspace.spaces[space].projects.is_empty() {
                                        self.workspace.selected_space = Some(space);
                                        self.workspace.selected_project = None;
                                        self.navigation = Navigation::Projects;
                                        Task::batch([self.load_directory(), self.save()])
                                    } else {
                                        self.update(Message::SelectProject(space, 0))
                                    }
                                }
                                _ => Task::none(),
                            }
                        }
                        _ => Task::none(),
                    };
                    tasks.push(task);
                }
                return Task::batch(tasks);
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
