mod settings;
mod style;

pub(crate) use settings::{connections_view, profiles_view, settings_dialog};

use super::{
    Operation,
    state::{Event, State},
};
use crate::{
    domain::deployment::{Connections, Error, Profile},
    i18n::{I18n, Key},
    ui::{
        context_menu,
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, button, column, container, mouse_area, row, scrollable, text},
};
use std::path::Path;

fn action<'a>(i18n: &I18n, key: Key, event: Event) -> Element<'a, Event> {
    button(text(i18n.text(key)).size(13))
        .on_press(event)
        .padding([8, 10])
        .style(theme::action(false))
        .into()
}

pub(crate) fn error_key(error: &Error) -> Key {
    match error {
        Error::Configuration => Key::DeploymentConfigurationError,
        Error::Mapping => Key::DeploymentMappingError,
        Error::Excluded => Key::DeploymentExcludedError,
        Error::Access => Key::DeploymentAccessError,
        Error::Connection => Key::DeploymentConnectionError,
        Error::Credentials => Key::DeploymentCredentialsError,
        Error::HostUnknown(_) => Key::DeploymentUnknownHost,
        Error::HostChanged => Key::DeploymentHostChanged,
        Error::Cancelled => Key::DeploymentCancelled,
        Error::TooLarge => Key::DeploymentTooLarge,
        Error::Binary => Key::DeploymentBinary,
        Error::Overwrite => Key::DeploymentOverwrite,
        Error::Changed => Key::DeploymentChanged,
        Error::Unsupported => Key::DeploymentUnsupported,
    }
}

pub(super) fn status<'a>(state: &State, i18n: &I18n) -> Element<'a, Event> {
    let mut content = column![].spacing(8);
    if !state.operation_name.is_empty() {
        content = content.push(text(state.operation_name.clone()).size(12));
    }
    if let Some(key) = state.operation_label {
        content = content.push(text(i18n.text(key)).size(13));
    }
    if !state.operation_path.is_empty() {
        content = content.push(text(state.operation_path.clone()).size(12));
    }
    if state.queued() > 0 {
        let mut args = crate::i18n::FluentArgs::new();
        args.set("count", state.queued());
        content = content.push(text(i18n.format(Key::DeploymentQueued, &args)).size(12));
    }
    if state.busy {
        content = content.push(text(i18n.text(Key::DeploymentWorking)).size(13));
        let bytes = state
            .progress
            .bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        let files = state
            .progress
            .files
            .load(std::sync::atomic::Ordering::Relaxed);
        content = content.push(
            row![
                text(i18n.text(Key::DeploymentFiles)).size(12),
                text(files.to_string()).size(12),
                text(i18n.text(Key::DeploymentBytes)).size(12),
                text(bytes.to_string()).size(12)
            ]
            .spacing(6),
        );
        content = content.push(action(i18n, Key::DeploymentCancel, Event::Cancel));
    } else if let Some(error) = &state.error {
        content = content.push(text(i18n.text(error_key(error))).size(13));
        match error {
            Error::HostUnknown(fingerprint) => {
                content = content.push(text(fingerprint.clone()).size(12));
                content = content.push(
                    row![
                        action(i18n, Key::DeploymentTrust, Event::TrustHost),
                        action(i18n, Key::DeploymentCancel, Event::Cancel)
                    ]
                    .spacing(6),
                );
            }
            Error::Overwrite => {
                content = content.push(
                    row![
                        action(i18n, Key::DeploymentReplace, Event::ConfirmDownload),
                        action(i18n, Key::DeploymentCancel, Event::Cancel)
                    ]
                    .spacing(6),
                )
            }
            Error::Configuration | Error::Mapping => {
                content = content.push(action(i18n, Key::DeploymentSettings, Event::Settings))
            }
            Error::HostChanged | Error::Excluded => {}
            _ => content = content.push(action(i18n, Key::DeploymentRetry, Event::Retry)),
        }
    } else if let Some(key) = state.status {
        content = content.push(text(i18n.text(key)).size(13));
    }
    content.into()
}

fn tree<'a>(state: &'a State, root: &str, depth: u16) -> Element<'a, Event> {
    let mut content = Column::new().spacing(2);
    if depth > 64 {
        return content.into();
    }
    if let Some(entries) = state.tree.get(root) {
        for entry in entries {
            if state.browsing && (!entry.directory || entry.symlink) {
                continue;
            }
            let symbol = if entry.directory {
                if state.expanded.contains(&entry.path) {
                    Icon::Down
                } else {
                    Icon::Right
                }
            } else {
                Icon::Changes
            };
            let arrow: Element<'_, Event> = if entry.directory && !entry.symlink {
                button(icon(symbol, theme::MUTED))
                    .padding(0)
                    .on_press(Event::Folder(entry.path.clone()))
                    .style(theme::action(false))
                    .into()
            } else {
                icon(symbol, theme::MUTED)
            };
            let entry_row = row![
                arrow,
                text(&entry.name).size(13).wrapping(text::Wrapping::None)
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            let selected = state
                .selected
                .as_ref()
                .is_some_and(|selected| selected.path == entry.path);
            let line = container(entry_row)
                .padding([5, 6])
                .width(Length::Fill)
                .style(if selected {
                    theme::badge
                } else {
                    theme::surface
                });
            let line: Element<'_, Event> = if entry.symlink {
                line.into()
            } else {
                let line = mouse_area(line)
                    .on_press(Event::Select(entry.clone()))
                    .on_double_click(if entry.directory {
                        Event::Folder(entry.path.clone())
                    } else {
                        Event::Run(Operation::Preview(entry.path.clone()))
                    });
                if state.browsing {
                    line.into()
                } else {
                    context_menu::trigger(line, move |point| Event::Menu(entry.clone(), point))
                }
            };
            content = content
                .push(container(line).padding(iced::Padding::ZERO.left(f32::from(depth) * 14.)));
            if entry.directory && state.expanded.contains(&entry.path) {
                content = content.push(tree(state, &entry.path, depth + 1));
            }
        }
    }
    content.into()
}

pub(crate) fn panel<'a>(
    state: &'a State,
    settings: &'a Connections,
    profiles: &'a [Profile],
    project: Option<&Path>,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut heading = column![
        text(i18n.text(Key::Deployment))
            .size(14)
            .font(theme::semibold())
    ]
    .spacing(6);
    let active = state.active_profile(profiles, project);
    if active.is_some() {
        heading = heading.push(settings::profile_selector(profiles, active, project));
    }
    if let Some(connection) =
        active.and_then(|p| settings.connections.iter().find(|c| c.id == p.connection))
    {
        heading = heading.push(text(&connection.name).size(12).style(theme::MUTED.text()));
    }
    let control = |symbol, key, event| {
        tooltip::view(
            button(icon(symbol, theme::MUTED))
                .on_press(event)
                .padding(7)
                .style(theme::action(false)),
            text(i18n.text(key)).size(12),
            tooltip::Position::Bottom,
        )
    };
    heading = heading.push(
        row![
            control(Icon::Refresh, Key::DeploymentRefresh, Event::Refresh),
            control(
                Icon::CollapseLines,
                Key::DeploymentCollapseAll,
                Event::CollapseAll
            ),
            control(Icon::Settings, Key::DeploymentSettings, Event::Settings),
            container(control(Icon::Minus, Key::HidePanel, Event::Toggle)).id("deployment-hide")
        ]
        .spacing(4),
    );
    let files: Element<'_, Event> = if let Some(profile) = active {
        tree(state, &profile.root, 0)
    } else {
        column![
            text(i18n.text(Key::DeploymentEmpty)).size(13),
            action(i18n, Key::DeploymentSettings, Event::Settings)
        ]
        .spacing(12)
        .into()
    };
    column![
        container(heading).padding(10),
        container(scrollable(files).style(theme::scroll)).height(Length::Fill)
    ]
    .height(Length::Fill)
    .into()
}

pub(crate) fn activity<'a>(state: &'a State, i18n: &'a I18n) -> Element<'a, Event> {
    let key = if state.busy {
        state.operation_label.unwrap_or(Key::DeploymentWorking)
    } else if let Some(error) = &state.error {
        error_key(error)
    } else {
        state.status.unwrap_or(Key::DeploymentComplete)
    };
    let mut controls = row![
        button(
            row![
                icon(Icon::Server, theme::MUTED),
                text(i18n.text(key)).size(12).wrapping(text::Wrapping::None),
                crate::ui::ellipsis::text(state.operation_name.as_str(), 12.)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .on_press(Event::ToggleActivity)
        .width(Length::Fill)
        .padding([4, 8])
        .style(theme::action(state.activity_expanded))
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if state.busy {
        let bytes = state
            .progress
            .bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        controls = controls
            .push(text(format!("{bytes}")).size(12))
            .push(text(i18n.text(Key::DeploymentBytes)).size(12))
            .push(
                button(icon(Icon::Close, theme::MUTED))
                    .on_press(Event::Cancel)
                    .padding(4)
                    .style(theme::action(false)),
            );
    }
    container(controls)
        .id("background-activity")
        .width(440)
        .clip(true)
        .into()
}

pub(crate) fn activity_details<'a>(state: &'a State, i18n: &'a I18n) -> Element<'a, Event> {
    column![
        row![
            text(i18n.text(Key::Deployment))
                .size(14)
                .width(Length::Fill),
            button(icon(Icon::Close, theme::MUTED))
                .on_press(Event::DismissActivity)
                .padding(4)
                .style(theme::action(false))
        ]
        .align_y(Alignment::Center),
        status(state, i18n)
    ]
    .spacing(10)
    .into()
}

pub(crate) fn overlay<'a, Message: Clone + 'a>(
    state: &'a State,
    i18n: &'a I18n,
    base: Element<'a, Message>,
    profiles: &'a [Profile],
    project: Option<&'a Path>,
    map: impl Fn(Event) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let Some(entry) = &state.menu else {
        return base;
    };
    context_menu::overlay(
        base,
        state.menu_point,
        iced::Size::new(280., if entry.directory { 135. } else { 265. }),
        move || {
            let mapped = state
                .active_profile(profiles, project)
                .is_some_and(|profile| profile.to_local(&entry.path).is_ok());
            let command = |key, event, enabled: bool| {
                button(text(i18n.text(key)).size(13))
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(theme::action(false))
                    .on_press_maybe(enabled.then_some(event))
            };
            let mut actions = column![
                command(
                    Key::DeploymentUpload,
                    Event::RemoteUpload(entry.path.clone()),
                    mapped
                ),
                command(
                    Key::DeploymentDownload,
                    Event::Run(Operation::Download(entry.path.clone(), false)),
                    mapped
                ),
            ]
            .spacing(4);
            if !entry.directory {
                actions = actions
                    .push(command(
                        Key::DeploymentCompareLocal,
                        Event::RemoteCompare(entry.path.clone()),
                        mapped,
                    ))
                    .push(command(
                        Key::DeploymentOpenLocal,
                        Event::OpenLocal(entry.path.clone()),
                        mapped,
                    ))
                    .push(command(
                        Key::DeploymentOpenRemote,
                        Event::Run(Operation::Preview(entry.path.clone())),
                        true,
                    ));
            }
            actions = actions.push(command(
                Key::CopyAbsolutePath,
                Event::CopyRemotePath(entry.path.clone()),
                true,
            ));
            Element::from(actions).map(map)
        },
        map(Event::DismissMenu),
    )
}

pub(crate) fn target_dialog<'a>(state: &'a State, i18n: &'a I18n) -> Option<Element<'a, Event>> {
    let (operation, choices) = state.target.as_ref()?;
    let title = if matches!(operation, Operation::Compare(_)) {
        Key::DeploymentCompare
    } else {
        Key::DeploymentUpload
    };
    let mut targets = column![].spacing(4);
    for (id, name) in choices {
        targets = targets.push(
            button(text(name).size(14))
                .width(Length::Fill)
                .padding(10)
                .style(theme::action(false))
                .on_press(Event::ChooseTarget(id.clone())),
        );
    }
    Some(
        column![
            text(i18n.text(title)).size(19),
            text(i18n.text(Key::DeploymentChooseTarget)).size(13),
            container(scrollable(targets).style(theme::scroll)).max_height(320),
            action(i18n, Key::DeploymentCancel, Event::CloseDialog)
        ]
        .spacing(12)
        .into(),
    )
}

#[cfg(test)]
mod tests;
