//! Keep settings and deployment in independent secondary windows.

use super::{App, Message};
use crate::{
    i18n::Key,
    infrastructure::window_effect,
    ui::{screens, theme},
};
use iced::{Element, Length, Size, Task, widget::container, window};

#[derive(Default)]
pub(crate) struct State {
    pub settings: Option<window::Id>,
    pub deployment: Option<window::Id>,
    pub connections: Option<window::Id>,
    #[cfg(target_os = "windows")]
    pub connections_maximized: bool,
    #[cfg(target_os = "windows")]
    pub deployment_maximized: bool,
    #[cfg(target_os = "windows")]
    pub maximized: bool,
}

impl State {
    pub(crate) fn is_secondary(&self, id: window::Id) -> bool {
        self.settings == Some(id) || self.is_deployment(id)
    }

    pub(crate) fn is_deployment(&self, id: window::Id) -> bool {
        self.deployment == Some(id) || self.connections == Some(id)
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn secondary_maximized(&self, id: window::Id) -> bool {
        if self.connections == Some(id) {
            self.connections_maximized
        } else if self.deployment == Some(id) {
            self.deployment_maximized
        } else {
            self.maximized
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Close(window::Id),
    Closed(window::Id),
    Configured,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Drag(window::Id),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    ToggleMaximized(window::Id),
    #[cfg(target_os = "windows")]
    Maximized(window::Id, bool),
    #[cfg(target_os = "windows")]
    Minimize(window::Id),
    #[cfg(target_os = "windows")]
    Resize(window::Id, window::Direction),
}

pub(super) fn main_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1440., 920.),
        min_size: Some(Size::new(1080., 720.)),
        position: window::Position::Centered,
        icon: crate::ui::branding::window_icon(),
        transparent: cfg!(target_os = "macos"),
        decorations: !cfg!(target_os = "windows"),
        exit_on_close_request: false,
        #[cfg(target_os = "macos")]
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..window::Settings::default()
    }
}

pub(super) fn settings_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1120., 760.),
        min_size: Some(Size::new(980., 660.)),
        position: window::Position::Centered,
        icon: crate::ui::branding::window_icon(),
        decorations: !cfg!(target_os = "windows"),
        transparent: cfg!(target_os = "macos"),
        exit_on_close_request: false,
        #[cfg(target_os = "macos")]
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..window::Settings::default()
    }
}

impl App {
    pub(super) fn open_settings(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.settings {
            return window::minimize::<Message>(id, false).chain(window::gain_focus(id));
        }
        let (id, open) = window::open(settings_settings());
        self.windows.settings = Some(id);
        #[cfg(target_os = "windows")]
        {
            self.windows.maximized = false;
        }
        open.map(Message::WindowOpened)
    }

    pub(super) fn open_deployment_settings(&mut self, connections: bool) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Deployment) {
            return Task::none();
        }
        let slot = if connections {
            &mut self.windows.connections
        } else {
            &mut self.windows.deployment
        };
        if let Some(id) = *slot {
            return window::minimize::<Message>(id, false).chain(window::gain_focus(id));
        }
        let (id, open) = window::open(settings_settings());
        *slot = Some(id);
        #[cfg(target_os = "windows")]
        if connections {
            self.windows.connections_maximized = false;
        } else {
            self.windows.deployment_maximized = false;
        }
        open.map(Message::WindowOpened)
    }

    pub(super) fn window_opened(&mut self, id: window::Id) -> Task<Message> {
        if self.windows.is_secondary(id) {
            return window::run(id, |window| {
                window_effect::apply(window);
            })
            .map(|()| Message::Window(Event::Configured));
        }
        if self.window_id.is_none() {
            self.window_id = Some(id);
        }
        if self.window_id != Some(id) {
            return Task::none();
        }
        Task::batch([
            window::run(id, window_effect::apply).map(Message::GlassConfigured),
            window::is_maximized(id).map(Message::WindowMaximized),
        ])
    }

    pub(super) fn update_window(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Close(id) if self.windows.is_secondary(id) => return window::close(id),
            Event::Close(id) if self.window_id == Some(id) => {
                if self.protect_editor(&Message::Window(Event::Close(id))) {
                    return self.flush_editor();
                }
                return iced::exit();
            }
            Event::Closed(id) if self.windows.settings == Some(id) => {
                self.windows.settings = None;
                self.shortcuts = super::shortcuts::State::default();
            }
            Event::Closed(id) if self.windows.is_deployment(id) => {
                let connections = self.windows.connections == Some(id);
                if connections {
                    self.windows.connections = None;
                } else {
                    self.windows.deployment = None;
                }
                return self.update_module(super::modules::Event::Deployment(
                    crate::modules::deployment::Event::CloseSettings(connections),
                ));
            }
            Event::Closed(id) if self.window_id == Some(id) => return iced::exit(),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Event::Drag(id) if self.windows.is_secondary(id) => return window::drag(id),
            #[cfg(target_os = "macos")]
            Event::ToggleMaximized(id) if self.windows.is_secondary(id) => {
                return window::toggle_maximize(id);
            }
            #[cfg(target_os = "windows")]
            Event::ToggleMaximized(id) if self.windows.is_secondary(id) => {
                return window::toggle_maximize::<Message>(id).chain(
                    window::is_maximized(id)
                        .map(move |value| Message::Window(Event::Maximized(id, value))),
                );
            }
            #[cfg(target_os = "windows")]
            Event::Maximized(id, value) if self.windows.connections == Some(id) => {
                self.windows.connections_maximized = value
            }
            #[cfg(target_os = "windows")]
            Event::Maximized(id, value) if self.windows.deployment == Some(id) => {
                self.windows.deployment_maximized = value
            }
            #[cfg(target_os = "windows")]
            Event::Maximized(id, value) if self.windows.settings == Some(id) => {
                self.windows.maximized = value
            }
            #[cfg(target_os = "windows")]
            Event::Minimize(id) if self.windows.is_secondary(id) => {
                return window::minimize(id, true);
            }
            #[cfg(target_os = "windows")]
            Event::Resize(id, direction)
                if self.windows.is_secondary(id) && !self.windows.secondary_maximized(id) =>
            {
                return window::drag_resize(id, direction);
            }
            _ => {}
        }
        Task::none()
    }

    pub(super) fn window_title(&self, id: window::Id) -> String {
        self.i18n.text(if self.windows.connections == Some(id) {
            Key::DeploymentConnections
        } else if self.windows.deployment == Some(id) {
            Key::DeploymentProfiles
        } else if self.windows.settings == Some(id) {
            Key::Settings
        } else {
            Key::AppTitle
        })
    }

    pub(super) fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if !self.windows.is_secondary(id) {
            return self.view();
        }
        let content = if self.windows.is_deployment(id) {
            screens::settings::deployment_view(self, self.windows.connections == Some(id))
        } else {
            screens::settings::view(self)
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let content: Element<'_, Message> = {
            use iced::widget::{column, mouse_area, text};
            let inset = if cfg!(target_os = "macos") { 90. } else { 16. };
            let header = mouse_area(
                container(
                    text(self.window_title(id))
                        .size(13)
                        .font(theme::semibold())
                        .style(theme::MUTED.text()),
                )
                .id("settings-window-header")
                .padding(iced::Padding {
                    left: inset,
                    right: inset,
                    ..iced::Padding::ZERO
                })
                .align_y(iced::alignment::Vertical::Center)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill)
                .height(theme::SETTINGS_HEADER_HEIGHT)
                .style(theme::sidebar),
            )
            .on_press(Message::Window(Event::Drag(id)))
            .on_double_click(Message::Window(Event::ToggleMaximized(id)));
            let border = container(iced::widget::space())
                .height(1)
                .width(Length::Fill)
                .style(theme::separator);
            column![header, border, content].height(Length::Fill).into()
        };
        #[cfg(target_os = "windows")]
        let content = crate::ui::window_chrome::settings_overlay(self, content, id);
        let content = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface)
            .into();
        // Quit decisions must remain reachable when a secondary window covers the workspace.
        crate::ui::editor::overlay(self, content)
    }
}

#[cfg(test)]
mod tests;
