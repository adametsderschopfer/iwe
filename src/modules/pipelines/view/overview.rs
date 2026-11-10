//! Show provider stages or individual GitHub jobs in a compact status chain.
use super::{Event as Message, View};
use crate::modules::pipelines::{
    PipelineState,
    jobs::{
        Event as JobsEvent,
        overview::{Event, Target},
    },
};

use super::{divider, icon_button, jobs, status};
use crate::ui::tooltip;
use crate::{
    domain::{
        Failure,
        jobs::{Action, Job, JobAction, Status},
        repository::{Pipeline, Provider},
    },
    i18n::{FluentArgs, Key},
    ui::{
        context_menu, ellipsis,
        icons::{Icon, icon},
        theme,
    },
};
use iced::{
    Alignment, Element, Length, Size,
    widget::{Column, Row, button, column, container, row, scrollable, space, text},
};

fn message(event: Event) -> Message {
    Message::Jobs(JobsEvent::Overview(event))
}

pub(super) fn chain<'a>(app: View<'a>, run: &'a Pipeline) -> Element<'a, Message> {
    let Some(entry) = app.state.jobs.overview.entries.get(&run.id) else {
        return text(app.i18n.text(Key::Loading))
            .size(11)
            .style(theme::FAINT.text())
            .into();
    };
    let Some(details) = &entry.details else {
        return if let Some(error) = entry.error {
            retry(app, run.id, error)
        } else {
            text(app.i18n.text(Key::Loading))
                .size(11)
                .style(theme::FAINT.text())
                .into()
        };
    };
    let github = app
        .remote
        .is_some_and(|remote| remote.provider == Some(Provider::GitHub));
    let mut nodes: Vec<Element<'a, Message>> = Vec::new();
    let mut count = 0;
    let mut push = |target: Target, name: &'a str, value: Status, id: String| {
        if count > 0 {
            nodes.push(
                container(space())
                    .width(theme::PIPELINE_CONNECTOR_WIDTH)
                    .height(2)
                    .style(theme::separator)
                    .into(),
            );
        }
        let selected = app
            .state
            .jobs
            .overview
            .popup
            .as_ref()
            .is_some_and(|popup| popup.pipeline == run.id && popup.target == target);
        let hint = column![
            text(name).size(12),
            text(app.i18n.text(status::label(value).0))
                .size(11)
                .style(theme::MUTED.text())
        ]
        .spacing(4);
        let node = context_menu::anchored(
            tooltip::view(
                container(status::badge(value, selected)).id(id),
                hint,
                tooltip::Position::Top,
            ),
            move |point| message(Event::Open(run.id, target.clone(), point)),
        );
        nodes.push(node);
        count += 1;
    };
    if github {
        for job in details.stages.iter().flat_map(|stage| &stage.jobs) {
            push(
                Target::Job(job.id),
                &job.name,
                Status::from_provider(&job.status),
                format!("pipeline-node-{}-job-{}", run.id, job.id),
            );
        }
    } else {
        for (index, stage) in details.stages.iter().enumerate() {
            if let Some(name) = &stage.name {
                push(
                    Target::Stage(name.clone()),
                    name,
                    stage.status(),
                    format!("pipeline-node-{}-stage-{index}", run.id),
                );
            }
        }
    }
    if count == 0 {
        return text(app.i18n.text(Key::NoJobs))
            .size(11)
            .style(theme::FAINT.text())
            .into();
    }
    let chain = scrollable(Row::with_children(nodes).align_y(Alignment::Center))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(3).scroller_width(3),
        ))
        .width(Length::Fill)
        .height(36)
        .style(theme::scroll);
    let mut content = column![chain].spacing(2);
    if let Some(error) = entry.error {
        content = content.push(retry(app, run.id, error));
    }
    content.into()
}

fn retry<'a>(app: View<'a>, id: u64, error: Failure) -> Element<'a, Message> {
    row![
        text(app.i18n.text(crate::ui::presentation::error_key(error)))
            .size(11)
            .style(theme::RED.text())
            .width(Length::Fill),
        container(icon_button(
            app.i18n,
            Icon::Refresh,
            Key::Refresh,
            message(Event::Refresh(id)),
            false
        ))
        .id(format!("pipeline-overview-retry-{id}")),
    ]
    .align_y(Alignment::Center)
    .spacing(4)
    .into()
}

pub(crate) fn overlay<'a>(app: View<'a>, base: Element<'a, Message>) -> Element<'a, Message> {
    if !app.visible
        || app.state.jobs.selected.is_some()
        || !matches!(app.state.pipelines, PipelineState::Ready(_))
    {
        return base;
    }
    let Some(popup) = &app.state.jobs.overview.popup else {
        return base;
    };
    let Some(entry) = app.state.jobs.overview.entries.get(&popup.pipeline) else {
        return base;
    };
    let Some(details) = &entry.details else {
        return base;
    };
    let (name, jobs, key) = match &popup.target {
        Target::Stage(name) => {
            let Some(stage) = details
                .stages
                .iter()
                .find(|stage| stage.name.as_ref() == Some(name))
            else {
                return base;
            };
            (
                name.as_str(),
                stage.jobs.iter().collect::<Vec<_>>(),
                Key::PipelineStageTitle,
            )
        }
        Target::Job(id) => {
            let Some(job) = details
                .stages
                .iter()
                .flat_map(|stage| &stage.jobs)
                .find(|job| job.id == *id)
            else {
                return base;
            };
            (job.name.as_str(), vec![job], Key::PipelineJobTitle)
        }
    };
    let mut args = FluentArgs::new();
    args.set("name", name);
    let title = app.i18n.format(key, &args);
    let rows_height = (jobs.len().max(1) as f32 * theme::PIPELINE_JOB_HEIGHT)
        .min(theme::PIPELINE_POPUP_JOBS_HEIGHT);
    let extra = if entry.error.is_some() || app.state.jobs.pending.is_some() {
        48.
    } else {
        0.
    };
    let height = rows_height + 104. + extra;
    context_menu::overlay(
        base,
        popup.position,
        Size::new(theme::PIPELINE_POPUP_WIDTH, height),
        move || {
            let heading = row![
                tooltip::view(
                    container(ellipsis::text(title.clone(), 13.)).width(Length::Fill),
                    text(title.clone()).size(12),
                    tooltip::Position::Top
                ),
                tooltip::view(
                    button(icon(Icon::Refresh, theme::MUTED))
                        .padding(5)
                        .style(theme::icon_action(false))
                        .on_press_maybe(
                            (!entry.is_loading() && app.state.jobs.pending.is_none())
                                .then_some(message(Event::Refresh(popup.pipeline)))
                        ),
                    text(app.i18n.text(Key::Refresh)).size(12),
                    tooltip::Position::Top
                ),
                icon_button(
                    app.i18n,
                    Icon::Close,
                    Key::Dismiss,
                    message(Event::Close),
                    false
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(6)
            .padding([2, 6]);
            let mut items = Column::new();
            if jobs.is_empty() {
                items = items.push(
                    container(
                        text(app.i18n.text(Key::NoStageJobs))
                            .size(12)
                            .style(theme::MUTED.text()),
                    )
                    .height(theme::PIPELINE_JOB_HEIGHT)
                    .center_y(Length::Fill),
                );
            }
            for &job in &jobs {
                items = items.push(job_row(app, popup.pipeline, job));
            }
            let mut body = column![
                heading,
                divider(),
                scrollable(items)
                    .width(Length::Fill)
                    .height(rows_height)
                    .style(theme::scroll)
            ]
            .spacing(6);
            if let Some(error) = entry.error {
                body = body.push(
                    text(app.i18n.text(crate::ui::presentation::error_key(error)))
                        .size(12)
                        .style(theme::RED.text()),
                );
            } else if app
                .state
                .jobs
                .pending
                .is_some_and(|(id, _)| id == popup.pipeline)
            {
                body = body.push(
                    text(app.i18n.text(Key::JobActionPending))
                        .size(12)
                        .style(theme::MUTED.text()),
                );
            }
            body = body.push(divider()).push(
                container(
                    button(text(app.i18n.text(Key::ViewPipelineJobs)).size(12))
                        .on_press(Message::Jobs(JobsEvent::Select(popup.pipeline)))
                        .padding([6, 10])
                        .style(theme::action(false)),
                )
                .id("pipeline-popup-details"),
            );
            container(body).id("pipeline-stage-popup").into()
        },
        message(Event::Close),
    )
}

fn job_row<'a>(app: View<'a>, pipeline: u64, job: &'a Job) -> Element<'a, Message> {
    let provider = app.remote.and_then(|remote| remote.provider);
    let value = Status::from_provider(&job.status);
    let mut item = row![
        tooltip::view(
            status::badge(value, false),
            text(app.i18n.text(status::label(value).0)).size(12),
            tooltip::Position::Top
        ),
        tooltip::view(
            button(ellipsis::text(&job.name, 13.))
                .width(Length::Fill)
                .style(theme::action(false))
                .padding([6, 2])
                .on_press(Message::OpenUrl(job.url.clone())),
            text(&job.name).size(12),
            tooltip::Position::Top
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    for action in &job.actions {
        let glyph = match action {
            JobAction::Play => Icon::Play,
            JobAction::Retry => Icon::Refresh,
            JobAction::Cancel => Icon::Close,
        };
        item = item.push(tooltip::view(
            container(
                button(icon(glyph, theme::MUTED))
                    .padding(5)
                    .style(theme::secondary)
                    .on_press_maybe((!app.job_controls_blocked(pipeline)).then_some(
                        Message::Jobs(JobsEvent::Control(pipeline, Action::Job(job.id, *action))),
                    )),
            )
            .id(format!("pipeline-popup-action-{}-{action:?}", job.id)),
            text(app.i18n.text(jobs::action_label(provider, *action))).size(12),
            tooltip::Position::Top,
        ));
    }
    container(item)
        .height(theme::PIPELINE_JOB_HEIGHT)
        .padding([0, 6])
        .id(format!("pipeline-popup-job-{}", job.id))
        .into()
}
