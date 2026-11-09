//! Keep hover hints separated from their controls throughout the interface.

pub(crate) use iced::widget::tooltip::Position;
use iced::{Element, widget::Tooltip};

pub(crate) fn view<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    hint: impl Into<Element<'a, Message>>,
    position: Position,
) -> Tooltip<'a, Message> {
    iced::widget::tooltip(content, hint, position)
        .gap(8)
        .style(super::theme::tooltip)
}

#[cfg(test)]
mod tests;
