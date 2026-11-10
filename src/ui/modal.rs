use crate::ui::theme;
use iced::{
    Element, Length,
    widget::{container, opaque, stack},
};

pub(crate) fn overlay<'a, Message: 'a>(
    base: Element<'a, Message>,
    content: Element<'a, Message>,
    width: f32,
) -> Element<'a, Message> {
    let dialog = container(content)
        .width(width)
        .padding(32)
        .style(theme::dialog);
    let backdrop = container(dialog)
        .center(Length::Fill)
        .padding(24)
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgba(0., 0., 0., 0.5).into()),
            ..Default::default()
        });
    stack![base, opaque(backdrop)].into()
}
