use crate::i18n::{I18n, Key};
use crate::ui::theme;
use iced::widget::{container, text};
use iced::{Element, Length};

pub(crate) fn view<'a, Message: 'a>(i18n: &'a I18n) -> Element<'a, Message> {
    container(text(i18n.text(Key::Greeting)).size(theme::GREETING_SIZE))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
