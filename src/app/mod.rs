//! Compose the greeting screen and native window.

use crate::i18n::{I18n, Key};
use crate::infrastructure::window_effect;
use crate::ui::{screens::hello_world, theme};
use iced::{Element, Program, Size, Task, Theme, window};
use std::rc::Rc;

pub(crate) fn run(i18n: Rc<I18n>) -> iced::Result {
    application(i18n).run()
}

fn application(
    i18n: Rc<I18n>,
) -> iced::Application<impl Program<State = App, Message = Message, Theme = Theme>> {
    iced::application(
        move || App {
            i18n: Rc::clone(&i18n),
            glass_ready: false,
        },
        App::update,
        App::view,
    )
    .title(App::title)
    .window(window::Settings {
        size: Size::new(800.0, 600.0),
        min_size: Some(Size::new(400.0, 280.0)),
        position: window::Position::Centered,
        transparent: cfg!(target_os = "macos"),
        ..window::Settings::default()
    })
    .subscription(|_| window::open_events().map(Message::WindowOpened))
    .theme(Theme::Dark)
    .style(|state, _| theme::application_style(state.glass_ready))
}

struct App {
    i18n: Rc<I18n>,
    glass_ready: bool,
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    GlassConfigured(bool),
}

impl App {
    fn title(&self) -> String {
        self.i18n.text(Key::AppTitle)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                return window::run(id, window_effect::apply).map(Message::GlassConfigured);
            }
            Message::GlassConfigured(ready) => self.glass_ready = ready,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        hello_world::view(&self.i18n)
    }
}

#[cfg(test)]
mod tests;
