//! Compose Windows caption controls and resize handles through safe Iced APIs.

use crate::{
    app::{App, Message, windows::Event},
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    mouse,
    widget::{Stack, button, container, mouse_area, row, space, text},
    window,
};

pub(crate) fn overlay<'a>(app: &'a App, content: Element<'a, Message>) -> Element<'a, Message> {
    compose(app, content, None, app.window_maximized)
}

pub(crate) fn settings_overlay<'a>(
    app: &'a App,
    content: Element<'a, Message>,
    id: window::Id,
) -> Element<'a, Message> {
    compose(app, content, Some(id), app.windows.maximized)
}

fn compose<'a>(
    app: &'a App,
    content: Element<'a, Message>,
    target: Option<window::Id>,
    maximized: bool,
) -> Element<'a, Message> {
    let edge = if maximized {
        0.
    } else {
        theme::WINDOW_RESIZE_GRIP
    };
    let content = container(content)
        .padding(iced::Padding {
            right: edge,
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface)
        .into();
    let caption = container(controls(app, target, maximized))
        .padding(iced::Padding {
            top: 8.,
            right: 24. + edge,
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Top);
    resize_edges(
        Stack::with_children(vec![content, caption.into()])
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        maximized,
        target,
    )
}

fn controls(app: &App, target: Option<window::Id>, maximized: bool) -> Element<'_, Message> {
    let (symbol, label) = if maximized {
        (Icon::Restore, Key::RestoreWindow)
    } else {
        (Icon::Maximize, Key::MaximizeWindow)
    };
    row![
        control(
            app,
            Icon::Minimize,
            Key::MinimizeWindow,
            target.map_or(Message::MinimizeWindow, |id| Message::Window(
                Event::Minimize(id)
            ))
        ),
        control(
            app,
            symbol,
            label,
            target.map_or(Message::ToggleWindowMaximized, |id| Message::Window(
                Event::ToggleMaximized(id)
            ))
        ),
        control(
            app,
            Icon::Close,
            Key::CloseWindow,
            target.map_or(Message::CloseWindow, |id| Message::Window(Event::Close(id)))
        ),
    ]
    .into()
}

fn control(app: &App, symbol: Icon, label: Key, message: Message) -> Element<'_, Message> {
    tooltip::view(
        button(container(icon(symbol, theme::TEXT)).center(Length::Fill))
            .on_press(message)
            .padding(0)
            .width(theme::WINDOW_CONTROL_WIDTH)
            .height(theme::WINDOW_CONTROL_HEIGHT)
            .style(theme::window_control),
        text(app.i18n.text(label)).size(12),
        tooltip::Position::Bottom,
    )
    .into()
}

fn resize_edges(
    content: Element<'_, Message>,
    maximized: bool,
    target: Option<window::Id>,
) -> Element<'_, Message> {
    use window::Direction;
    let grip = Length::Fixed(if maximized {
        0.
    } else {
        theme::WINDOW_RESIZE_GRIP
    });
    let mut layers = vec![content];
    for (direction, horizontal, vertical, width, height, cursor) in [
        (
            Direction::North,
            Horizontal::Center,
            Vertical::Top,
            Length::Fill,
            grip,
            mouse::Interaction::ResizingVertically,
        ),
        (
            Direction::South,
            Horizontal::Center,
            Vertical::Bottom,
            Length::Fill,
            grip,
            mouse::Interaction::ResizingVertically,
        ),
        (
            Direction::West,
            Horizontal::Left,
            Vertical::Center,
            grip,
            Length::Fill,
            mouse::Interaction::ResizingHorizontally,
        ),
        (
            Direction::East,
            Horizontal::Right,
            Vertical::Center,
            grip,
            Length::Fill,
            mouse::Interaction::ResizingHorizontally,
        ),
        (
            Direction::NorthWest,
            Horizontal::Left,
            Vertical::Top,
            grip,
            grip,
            mouse::Interaction::ResizingDiagonallyDown,
        ),
        (
            Direction::NorthEast,
            Horizontal::Right,
            Vertical::Top,
            grip,
            grip,
            mouse::Interaction::ResizingDiagonallyUp,
        ),
        (
            Direction::SouthWest,
            Horizontal::Left,
            Vertical::Bottom,
            grip,
            grip,
            mouse::Interaction::ResizingDiagonallyUp,
        ),
        (
            Direction::SouthEast,
            Horizontal::Right,
            Vertical::Bottom,
            grip,
            grip,
            mouse::Interaction::ResizingDiagonallyDown,
        ),
    ] {
        let handle = mouse_area(container(space()).width(width).height(height))
            .on_press(target.map_or(Message::ResizeWindow(direction), |id| {
                Message::Window(Event::Resize(id, direction))
            }))
            .interaction(cursor);
        layers.push(
            container(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(horizontal)
                .align_y(vertical)
                .into(),
        );
    }
    Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
