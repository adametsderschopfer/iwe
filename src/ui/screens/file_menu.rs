//! Offer direct absolute-path copying and a hover submenu for path formats.

use crate::{
    app::{
        App, Message,
        files::{Event, PathFormat},
    },
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length, Point,
    widget::{button, column, container, mouse_area, opaque, responsive, row, space, stack, text},
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(menu) = &app.files.menu else {
        return base;
    };
    let relative_key = if app.path_repository_root(&menu.path).is_some() {
        Key::CopyRepositoryPath
    } else {
        Key::CopyProjectPath
    };
    let popup = responsive(move |available| {
        let width = 200_f32.min(available.width);
        let child_width = 300_f32.min(available.width);
        let origin = Point::new(
            menu.point.x.clamp(0., (available.width - width).max(0.)),
            menu.point.y.clamp(0., (available.height - 58.).max(0.)),
        );
        let main = mouse_area(
            button(
                row![
                    text(app.i18n.text(Key::CopyPathMenu)).size(14),
                    space().width(Length::Fill),
                    icon(Icon::Right, theme::MUTED),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(Message::Files(Event::CopyPath(PathFormat::Absolute)))
            .padding([10, 12])
            .width(Length::Fill)
            .style(theme::action(false)),
        )
        .on_enter(Message::Files(Event::CopyOptions));
        let mut layers = vec![panel(main.into(), origin, width)];
        if menu.copy_open {
            let x = if origin.x + width + child_width <= available.width {
                origin.x + width
            } else {
                (origin.x - child_width).max(0.)
            };
            let position = Point::new(x, origin.y.min((available.height - 104.).max(0.)));
            let option = |key, format| {
                button(
                    text(app.i18n.text(key))
                        .size(13)
                        .wrapping(text::Wrapping::None),
                )
                .on_press(Message::Files(Event::CopyPath(format)))
                .padding([10, 12])
                .width(Length::Fill)
                .style(theme::action(false))
            };
            let children = column![
                option(Key::CopyAbsolutePath, PathFormat::Absolute),
                option(relative_key, PathFormat::Relative),
            ]
            .spacing(4);
            layers.push(panel(children.into(), position, child_width));
        }
        iced::widget::Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    });
    stack![
        base,
        opaque(
            mouse_area(popup)
                .on_press(Message::Files(Event::DismissMenu))
                .on_right_press(Message::Files(Event::DismissMenu))
        )
    ]
    .into()
}

fn panel<'a>(content: Element<'a, Message>, point: Point, width: f32) -> Element<'a, Message> {
    container(opaque(
        container(content)
            .padding(8)
            .width(width)
            .style(theme::tooltip),
    ))
    .padding(iced::Padding {
        left: point.x,
        top: point.y,
        ..iced::Padding::ZERO
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
