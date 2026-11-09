//! Present a compact Go to Line dialog above the source preview.

use crate::{
    app::{App, Message, line_navigation::Event},
    i18n::Key,
    ui::theme,
};
use iced::{
    Element, Length,
    widget::{button, column, row, space, text, text_input},
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(dialog) = &app.line_navigation else {
        return base;
    };
    let input = text_input(&app.i18n.text(Key::LineNumber), &dialog.value)
        .id("go-to-line-number")
        .padding([10, 12])
        .size(14)
        .style(theme::input);
    let input = if dialog.loading {
        input
    } else {
        input
            .on_input(|value| Message::GoToLine(Event::Changed(value)))
            .on_submit(Message::GoToLine(Event::Submit))
    };
    let mut content = column![
        text(app.i18n.text(Key::GoToLine))
            .size(20)
            .font(theme::semibold()),
        text(app.i18n.text(Key::GoToLineDescription))
            .size(13)
            .style(theme::MUTED.text()),
        input,
    ]
    .spacing(14);
    if let Some(error) = dialog.error {
        content = content.push(text(app.i18n.text(error)).size(13).style(theme::RED.text()));
    }
    content = content.push(
        row![
            space().width(Length::Fill),
            button(text(app.i18n.text(Key::Cancel)).size(14))
                .on_press(Message::GoToLine(Event::Cancel))
                .padding([8, 14])
                .style(theme::action(false)),
            button(
                text(app.i18n.text(if dialog.loading {
                    Key::Loading
                } else {
                    Key::GoToLine
                }))
                .size(14)
            )
            .on_press_maybe((!dialog.loading).then_some(Message::GoToLine(Event::Submit)))
            .padding([8, 14])
            .style(theme::primary),
        ]
        .spacing(8),
    );
    super::modal::overlay(base, content.into(), 420.)
}
