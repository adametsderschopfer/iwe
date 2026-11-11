use crate::{
    app::{
        App, Message,
        auth::{Message as AuthMessage, Stage},
        error_key,
    },
    domain::auth::Secret,
    i18n::Key,
    ui::theme,
};
use iced::{
    Alignment, Element, Font, Length,
    widget::{button, column, row, space, text, text_input},
};

pub(super) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(dialog) = &app.auth_dialog else {
        return base;
    };
    let mut content = column![
        text(app.i18n.text(Key::SignIn))
            .size(24)
            .font(theme::semibold()),
        text(format!(
            "{} · {}",
            dialog.account.provider.name(),
            dialog.account.host
        ))
        .size(14)
        .style(theme::MUTED.text()),
    ]
    .spacing(16);
    match dialog.stage {
        Stage::Starting | Stage::Connecting => {
            content = content.push(text(app.i18n.text(Key::Connecting)).size(14));
        }
        Stage::Waiting => {
            content = content.push(text(app.i18n.text(Key::BrowserWaiting)).size(14));
            if let Some(prompt) = &dialog.prompt {
                if let Some(code) = &prompt.code {
                    content = content
                        .push(
                            text(app.i18n.text(Key::DeviceCodeDescription))
                                .size(13)
                                .style(theme::MUTED.text()),
                        )
                        .push(
                            row![
                                text(code).font(Font::MONOSPACE).size(28),
                                space().width(Length::Fill),
                                button(text(app.i18n.text(Key::CopyAuthCode)).size(12))
                                    .padding([8, 12])
                                    .style(theme::action(false))
                                    .on_press(Message::Auth(AuthMessage::CopyCode)),
                            ]
                            .align_y(Alignment::Center),
                        );
                }
                content = content.push(
                    button(text(app.i18n.text(Key::OpenBrowser)).size(14))
                        .padding([12, 20])
                        .style(theme::primary)
                        .on_press(Message::OpenUrl(prompt.url.clone())),
                );
            }
        }
        Stage::Token => {
            content = content
                .push(
                    text(app.i18n.text(
                        if dialog.account.provider == crate::domain::repository::Provider::GitLab {
                            Key::TokenDescription
                        } else {
                            Key::GitHubTokenDescription
                        },
                    ))
                    .size(14)
                    .line_height(1.5)
                    .style(theme::MUTED.text()),
                )
                .push(text(app.i18n.text(Key::TokenLabel)).size(14))
                .push(
                    text_input(&app.i18n.text(Key::TokenPlaceholder), dialog.token.expose())
                        .secure(true)
                        .padding([12, 16])
                        .size(14)
                        .style(theme::input)
                        .on_input(|value| {
                            Message::Auth(AuthMessage::TokenChanged(Secret::new(value)))
                        })
                        .on_submit(Message::Auth(AuthMessage::SubmitToken)),
                );
            let token_path =
                if dialog.account.provider == crate::domain::repository::Provider::GitLab {
                    "/-/user_settings/personal_access_tokens"
                } else {
                    "/settings/tokens"
                };
            content = content.push(
                button(text(app.i18n.text(Key::CreateToken)).size(12))
                    .style(theme::action(false))
                    .padding([8, 0])
                    .on_press(Message::OpenUrl(format!(
                        "{}{token_path}",
                        dialog.account.base_url()
                    ))),
            );
        }
        Stage::Failed => {}
    }
    if let Some(error) = dialog.error {
        content = content.push(
            text(app.i18n.text(error_key(error)))
                .size(13)
                .style(theme::RED.text()),
        );
    }
    let mut actions = row![
        space().width(Length::Fill),
        button(text(app.i18n.text(Key::CancelSignIn)).size(14))
            .padding([12, 20])
            .style(theme::secondary)
            .on_press(Message::Auth(AuthMessage::Cancel))
    ]
    .spacing(8);
    if dialog.stage == Stage::Token {
        actions = actions.push(
            button(text(app.i18n.text(Key::SignIn)).size(14))
                .padding([12, 20])
                .style(theme::primary)
                .on_press_maybe(
                    (!dialog.token.is_empty()).then_some(Message::Auth(AuthMessage::SubmitToken)),
                ),
        );
    } else if dialog.stage == Stage::Failed {
        actions = actions.push(
            button(text(app.i18n.text(Key::Retry)).size(14))
                .padding([12, 20])
                .style(theme::primary)
                .on_press(Message::SignIn),
        );
    }
    content = content.push(space().height(4)).push(actions);
    super::modal::overlay(
        base,
        content.into(),
        520.,
        Message::Auth(AuthMessage::Cancel),
    )
}
