use crate::domain::Failure;
use crate::{
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{button, container, space, text},
};
pub(crate) fn icon_button<'a, Message: Clone + 'a>(
    i18n: &crate::i18n::I18n,
    symbol: Icon,
    label: Key,
    message: Message,
    active: bool,
) -> Element<'a, Message> {
    tooltip::view(
        button(
            container(icon(
                symbol,
                if active { theme::TEXT } else { theme::MUTED },
            ))
            .center(32),
        )
        .on_press(message)
        .padding(4)
        .style(theme::icon_action(active)),
        text(i18n.text(label)).size(12),
        tooltip::Position::Bottom,
    )
    .into()
}

pub(crate) fn divider<'a, Message: 'a>() -> Element<'a, Message> {
    container(space())
        .height(1)
        .width(Length::Fill)
        .style(theme::separator)
        .into()
}

pub(crate) fn center_message<'a, Message: 'a>(
    i18n: &crate::i18n::I18n,
    key: Key,
) -> Element<'a, Message> {
    container(
        text(i18n.text(key))
            .size(14)
            .style(theme::MUTED.text())
            .align_x(Alignment::Center),
    )
    .padding(20)
    .center(Length::Fill)
    .into()
}

pub(crate) fn short(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.to_owned()
    } else {
        format!("{}…", value.chars().take(limit).collect::<String>())
    }
}

pub(crate) fn error_key(error: Failure) -> Key {
    match error {
        Failure::MissingTool => Key::GitMissing,
        Failure::NotRepository => Key::NotRepository,
        Failure::Read => Key::ReadFailed,
        Failure::TooLarge => Key::TooLarge,
        Failure::Binary => Key::Binary,
        Failure::Access => Key::AccessRequired,
        Failure::Unavailable => Key::Unavailable,
        Failure::Storage => Key::StorageSaveFailed,
        Failure::Credentials => Key::CredentialsUnavailable,
        Failure::AuthNotConfigured => Key::AuthNotConfigured,
        Failure::AuthCancelled => Key::Cancelled,
        Failure::AuthExpired => Key::AuthExpired,
        Failure::AuthDenied => Key::AuthDenied,
        Failure::AuthPortBusy => Key::AuthPortBusy,
        Failure::InvalidToken => Key::InvalidToken,
        Failure::PermissionDenied => Key::PermissionDenied,
        Failure::RateLimited => Key::RateLimited,
    }
}
