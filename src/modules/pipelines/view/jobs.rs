use super::{Event as Message, View};
use super::{center_message, divider, icon_button, short, status};
use crate::domain::{
    Failure,
    jobs::{Action, JobAction},
    repository::Provider,
};
use crate::modules::pipelines::jobs::Event;
use crate::ui::presentation::error_key;
use crate::ui::tooltip;
use crate::{
    i18n::Key,
    ui::{
        ellipsis, file_icons,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, row, scrollable, space, text},
};

pub(crate) fn view<'a>(app: View<'a>) -> Element<'a, Message> {
    let Some(run) = &app.state.jobs.selected else {
        return space().into();
    };
    let title = if run.title.is_empty() {
        format!("#{}", run.id)
    } else {
        run.title.clone()
    };
    let provider = app.remote.and_then(|remote| remote.provider);
    let busy = app.state.jobs.pending.is_some()
        || app.disconnecting
        || app.state.jobs.error.is_some()
        || app.state.jobs.action_error.is_some();
    let mut heading = row![
        container(
            button(
                row![
                    icon(Icon::Back, theme::MUTED),
                    text(app.i18n.text(Key::BackToPipelines)).size(12)
                ]
                .spacing(6)
                .align_y(Alignment::Center)
            )
            .on_press(Message::Jobs(Event::Back))
            .padding([6, 8])
            .style(theme::action(false))
        )
        .id("jobs-back"),
        text(short(&title, 60))
            .size(13)
            .font(theme::semibold())
            .wrapping(text::Wrapping::None)
            .width(Length::Fill),
        icon_button(
            app.i18n,
            Icon::Refresh,
            Key::Refresh,
            Message::Jobs(Event::Refresh),
            false
        ),
        icon_button(
            app.i18n,
            Icon::External,
            Key::OpenBrowser,
            Message::OpenUrl(run.url.clone()),
            false
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if app
        .state
        .jobs
        .details
        .as_ref()
        .is_some_and(|details| details.cancelable)
    {
        heading = heading.push(
            container(
                button(text(app.i18n.text(Key::CancelPipeline)).size(12))
                    .padding([6, 10])
                    .style(theme::action(false))
                    .on_press_maybe((!busy).then_some(Message::Jobs(Event::Control(
                        run.id,
                        Action::CancelPipeline,
                    )))),
            )
            .id("cancel-pipeline"),
        );
    }
    let mut body = Column::new().spacing(6);
    if app.state.jobs.pending.is_some() {
        body = body.push(
            text(app.i18n.text(Key::JobActionPending))
                .size(12)
                .style(theme::MUTED.text()),
        );
    }
    if let Some(error) = app.state.jobs.action_error.or(app.state.jobs.error) {
        let access = matches!(
            error,
            Failure::Access
                | Failure::AuthExpired
                | Failure::InvalidToken
                | Failure::PermissionDenied
        );
        let key = if app.state.jobs.action_error.is_some() && access {
            Key::JobActionAccess
        } else if error == Failure::TooLarge {
            Key::TooManyJobs
        } else {
            error_key(error)
        };
        body = body.push(
            row![
                text(app.i18n.text(key))
                    .size(12)
                    .style(theme::RED.text())
                    .width(Length::Fill),
                button(
                    text(
                        app.i18n
                            .text(if access { Key::SignIn } else { Key::Refresh })
                    )
                    .size(12)
                )
                .padding([6, 10])
                .style(theme::secondary)
                .on_press(if access {
                    Message::SignIn
                } else {
                    Message::Jobs(Event::Refresh)
                }),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }
    if let Some(details) = &app.state.jobs.details {
        if details.stages.iter().all(|stage| stage.jobs.is_empty()) {
            body = body.push(
                text(app.i18n.text(Key::NoJobs))
                    .size(13)
                    .style(theme::MUTED.text()),
            );
        }
        for (index, stage) in details.stages.iter().enumerate() {
            let stage_key = stage.name.clone().unwrap_or_default();
            let expanded = !app.state.jobs.collapsed.contains(&stage_key);
            body = body.push(
                container(
                    button(
                        row![
                            file_icons::chevron(expanded),
                            text(
                                stage
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| app.i18n.text(Key::Jobs))
                            )
                            .size(13)
                            .font(theme::semibold()),
                            text(stage.jobs.len()).size(11).style(theme::FAINT.text()),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 4])
                    .width(Length::Fill)
                    .style(theme::action(false))
                    .on_press(Message::Jobs(Event::ToggleStage(stage_key))),
                )
                .id(format!("job-stage-{index}")),
            );
            if !expanded {
                continue;
            }
            for job in &stage.jobs {
                let (key, color) = status(&job.status);
                let mut item = row![
                    text(app.i18n.text(key))
                        .size(12)
                        .style(color.text())
                        .width(130),
                    tooltip::view(
                        container(ellipsis::text(&job.name, 12.)).width(Length::Fill),
                        text(&job.name).size(12),
                        tooltip::Position::Top,
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center);
                for action in &job.actions {
                    let label = action_label(provider, *action);
                    item = item.push(
                        container(
                            button(text(app.i18n.text(label)).size(12))
                                .padding([6, 10])
                                .style(theme::action(false))
                                .on_press_maybe((!busy).then_some(Message::Jobs(Event::Control(
                                    run.id,
                                    Action::Job(job.id, *action),
                                )))),
                        )
                        .id(format!("job-action-{}-{action:?}", job.id)),
                    );
                }
                item = item.push(icon_button(
                    app.i18n,
                    Icon::External,
                    Key::OpenBrowser,
                    Message::OpenUrl(job.url.clone()),
                    false,
                ));
                body = body.push(
                    container(item)
                        .padding([0, 24])
                        .id(format!("job-{}", job.id)),
                );
            }
        }
    } else if app.state.jobs.error.is_none() {
        return column![
            container(heading).padding([8, 16]),
            divider(),
            center_message(app.i18n, Key::Loading)
        ]
        .height(Length::Fill)
        .into();
    }
    column![
        container(heading).padding([8, 16]),
        divider(),
        scrollable(container(body).padding([8, 16]))
            .id("pipeline-jobs")
            .style(theme::scroll)
            .height(Length::Fill),
    ]
    .height(Length::Fill)
    .into()
}

pub(super) fn action_label(provider: Option<Provider>, action: JobAction) -> Key {
    match action {
        JobAction::Play => Key::RunJob,
        JobAction::Retry if provider == Some(Provider::GitHub) => Key::RetryJobWithDependents,
        JobAction::Retry => Key::RetryJob,
        JobAction::Cancel => Key::CancelJob,
    }
}
