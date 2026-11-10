//! Place background operation details above the shared status bar.

use super::theme;
use iced::{
    Element, Length,
    widget::{container, opaque, scrollable, stack},
};

pub(crate) fn overlay<'a, Message: 'a>(
    base: Element<'a, Message>,
    details: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let Some(details) = details else {
        return base;
    };
    stack![
        base,
        container(opaque(
            container(scrollable(details).style(theme::scroll))
                .padding(12)
                .width(380)
                .max_height(360)
                .style(theme::tooltip)
        ))
        .padding(iced::Padding::ZERO.right(16.).bottom(44.))
        .align_right(Length::Fill)
        .align_bottom(Length::Fill)
    ]
    .into()
}
