use super::workspace::{center_message, divider, icon_button, short};
use crate::{
    app::{App, Message, Panel, PipelineState, auth::Message as AuthMessage, error_key},
    domain::{Failure, repository::Provider},
    i18n::Key,
    ui::{
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, row, scrollable, space, text},
};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let Some(remote) = app
        .repository
        .as_ref()
        .and_then(|repo| repo.remote.as_ref())
    else {
        return space().into();
    };
    let provider = remote.provider.map(Provider::name).unwrap_or(&remote.host);
    let account: Element<'_, Message> = if remote.provider.is_none() {
        space().into()
    } else {
        row![
            text(short(
                app.account_username.as_deref().unwrap_or_default(),
                16
            ))
            .size(11)
            .color(theme::MUTED),
            button(
                text(app.i18n.text(if app.account_username.is_some() {
                    Key::Disconnect
                } else {
                    Key::SignIn
                }))
                .size(12)
            )
            .style(theme::action(false))
            .padding([6, 10])
            .on_press_maybe((!app.disconnecting).then_some(
                if app.account_username.is_some() {
                    Message::Auth(AuthMessage::Disconnect)
                } else {
                    Message::SignIn
                }
            )),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    };
    let header = row![
        icon(Icon::Play, theme::MUTED),
        text(app.i18n.text(Key::Pipelines))
            .size(14)
            .font(theme::semibold()),
        text(provider).size(12).color(theme::FAINT),
        space().width(Length::Fill),
        text(short(&remote.project, 24))
            .size(12)
            .color(theme::MUTED),
        account,
        icon_button(
            app,
            Icon::Refresh,
            Key::Refresh,
            Message::RefreshPipelines,
            false
        ),
        icon_button(
            app,
            Icon::Close,
            Key::HidePanel,
            Message::TogglePanel(Panel::Pipelines),
            false
        )
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let content: Element<'_, Message> = if app.jobs.selected.is_some() {
        jobs::view(app)
    } else if remote.provider.is_none() {
        container(
            column![
                text(app.i18n.text(Key::ChooseProvider)).size(14),
                row![
                    button(text("GitHub").size(13))
                        .on_press(Message::ChooseProvider(Provider::GitHub))
                        .padding([8, 16])
                        .style(theme::secondary),
                    button(text("GitLab").size(13))
                        .on_press(Message::ChooseProvider(Provider::GitLab))
                        .padding([8, 16])
                        .style(theme::secondary)
                ]
                .spacing(8)
            ]
            .spacing(12)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .into()
    } else {
        match &app.pipelines {
            PipelineState::Loading => center_message(app, Key::Loading),
            PipelineState::Idle => center_message(app, Key::Loading),
            PipelineState::Failed(error) => {
                let access = matches!(
                    error,
                    Failure::Access
                        | Failure::AuthExpired
                        | Failure::InvalidToken
                        | Failure::PermissionDenied
                );
                let action = if access {
                    Message::SignIn
                } else {
                    Message::RefreshPipelines
                };
                container(
                    row![
                        column![
                            text(app.i18n.text(error_key(*error))).size(14),
                            text(app.i18n.text(if access {
                                Key::SignInDescription
                            } else {
                                Key::RetryDescription
                            }))
                            .size(12)
                            .color(theme::MUTED)
                        ]
                        .spacing(6)
                        .width(Length::Fill),
                        button(
                            text(app.i18n.text(if access { Key::SignIn } else { Key::Retry }))
                                .size(13)
                        )
                        .on_press(action)
                        .padding([10, 18])
                        .style(theme::primary)
                    ]
                    .spacing(20)
                    .align_y(Alignment::Center),
                )
                .padding(24)
                .center_y(Length::Fill)
                .into()
            }
            PipelineState::Ready(runs) if runs.is_empty() => center_message(app, Key::NoPipelines),
            PipelineState::Ready(runs) => {
                let mut rows = Column::new().spacing(2);
                for run in runs {
                    let (key, color) = status(&run.status);
                    let title = if run.title.is_empty() {
                        format!("#{}", run.id)
                    } else {
                        run.title.clone()
                    };
                    let item = row![
                        text(app.i18n.text(key)).size(12).color(color).width(130),
                        text(short(&title, 75)).size(12).width(Length::Fill),
                        icon(Icon::Branch, theme::MUTED),
                        text(short(&run.branch, 24))
                            .size(11)
                            .color(theme::MUTED)
                            .width(180),
                        text(short(&run.date.replace('T', " "), 16))
                            .size(11)
                            .color(theme::FAINT)
                            .width(140),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center);
                    rows = rows.push(
                        row![
                            container(
                                button(item)
                                    .padding([10, 24])
                                    .width(Length::Fill)
                                    .style(theme::action(false))
                                    .on_press(Message::Jobs(crate::app::jobs::Event::Select(
                                        run.id
                                    )))
                            )
                            .id(format!("pipeline-{}", run.id))
                            .width(Length::Fill),
                            icon_button(
                                app,
                                Icon::External,
                                Key::OpenBrowser,
                                Message::OpenUrl(run.url.clone()),
                                false
                            ),
                        ]
                        .align_y(Alignment::Center)
                        .padding(iced::Padding {
                            right: 12.,
                            ..Default::default()
                        }),
                    );
                }
                scrollable(rows)
                    .style(theme::scroll)
                    .height(Length::Fill)
                    .into()
            }
        }
    };
    container(column![
        container(header).padding([8, 24]),
        divider(),
        content
    ])
    .height(Length::Fill)
    .style(theme::panel)
    .into()
}

fn status(value: &str) -> (Key, iced::Color) {
    match value {
        "success" | "passed" => (Key::Passed, theme::GREEN),
        "failure" | "failed" | "timed_out" | "startup_failure" | "action_required" => {
            (Key::Failed, theme::RED)
        }
        "running" | "in_progress" => (Key::Running, theme::BLUE),
        "canceling" => (Key::Canceling, theme::YELLOW),
        "queued"
        | "pending"
        | "waiting"
        | "waiting_for_resource"
        | "waiting_for_callback"
        | "requested"
        | "created"
        | "preparing"
        | "scheduled" => (Key::Pending, theme::YELLOW),
        "cancelled" | "canceled" => (Key::Cancelled, theme::MUTED),
        "skipped" | "neutral" => (Key::Skipped, theme::MUTED),
        "manual" => (Key::Manual, theme::YELLOW),
        _ => (Key::UnknownStatus, theme::MUTED),
    }
}
mod jobs;
