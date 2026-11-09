use super::workspace::{center_message, divider, icon_button, short};
use crate::{
    app::{App, Message, Panel, PipelineState, auth::Message as AuthMessage, error_key},
    domain::{
        Failure,
        jobs::Status,
        repository::{Pipeline, Provider},
    },
    i18n::Key,
    ui::{
        ellipsis,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, row, scrollable, space, text, tooltip},
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
            .style(theme::MUTED.text()),
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
        text(provider).size(12).style(theme::FAINT.text()),
        space().width(Length::Fill),
        text(short(&remote.project, 24))
            .size(12)
            .style(theme::MUTED.text()),
        account,
        icon_button(
            app,
            Icon::Refresh,
            Key::Refresh,
            Message::Jobs(crate::app::jobs::Event::Overview(
                crate::app::jobs::overview::Event::RefreshVisible
            )),
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
                            .style(theme::MUTED.text())
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
                let mut rows = Column::new();
                for run in runs {
                    rows = rows.push(pipeline_row(app, run));
                }
                scrollable(rows)
                    .id("pipelines-list")
                    .width(Length::Fill)
                    .style(theme::scroll)
                    .height(Length::Fill)
                    .on_scroll(|viewport| {
                        let start = (viewport.absolute_offset().y / theme::PIPELINE_ROW_HEIGHT)
                            .floor() as usize;
                        let end = ((viewport.absolute_offset().y + viewport.bounds().height)
                            / theme::PIPELINE_ROW_HEIGHT)
                            .ceil() as usize
                            + 1;
                        Message::Jobs(crate::app::jobs::Event::Overview(
                            crate::app::jobs::overview::Event::Viewport(start..end),
                        ))
                    })
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

fn status(value: &str) -> (Key, theme::Tone) {
    status::label(Status::from_provider(value))
}

fn pipeline_row<'a>(app: &'a App, run: &'a Pipeline) -> Element<'a, Message> {
    let title = if run.title.is_empty() {
        format!("#{}", run.id)
    } else {
        run.title.clone()
    };
    let mut metadata = row![
        icon(Icon::Branch, theme::MUTED),
        container(ellipsis::text(&run.branch, 11.)).width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(commit) = &run.commit {
        metadata = metadata.push(
            tooltip(
                container(
                    text(commit.chars().take(8).collect::<String>())
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .style(theme::MUTED.text()),
                )
                .id(format!("pipeline-commit-{}", run.id)),
                text(commit).size(12),
                tooltip::Position::Top,
            )
            .style(theme::tooltip),
        );
    }
    metadata = metadata.push(
        text(short(&run.date.replace('T', " "), 16))
            .size(11)
            .style(theme::FAINT.text()),
    );
    let info = column![ellipsis::text(title, 13.), metadata]
        .spacing(4)
        .width(Length::Fill);
    let summary = row![
        tooltip(
            status::badge(Status::from_provider(&run.status), false),
            text(app.i18n.text(status(&run.status).0)).size(12),
            tooltip::Position::Top
        )
        .style(theme::tooltip),
        info,
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    column![
        row![
            container(
                button(summary)
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(theme::action(false))
                    .on_press(Message::OpenUrl(run.url.clone()))
            )
            .id(format!("pipeline-{}", run.id))
            .width(Length::FillPortion(5)),
            container(overview::chain(app, run)).width(Length::FillPortion(4)),
            icon_button(
                app,
                Icon::External,
                Key::OpenBrowser,
                Message::OpenUrl(run.url.clone()),
                false
            ),
        ]
        .height(theme::PIPELINE_ROW_HEIGHT - 1.)
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([0, 12]),
        divider(),
    ]
    .height(theme::PIPELINE_ROW_HEIGHT)
    .into()
}

mod jobs;
mod overview;
mod status;
pub(super) use overview::overlay;
