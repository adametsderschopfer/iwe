use super::{
    Operation,
    state::{Event, Field, State},
    transport::Secret,
};
use crate::{
    domain::deployment::{Authentication, Connections, Error, Profile, Protocol},
    i18n::{I18n, Key},
    ui::{
        context_menu,
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, button, column, container, mouse_area, row, scrollable, text, text_input, toggler,
    },
};
use std::path::Path;

fn action<'a>(i18n: &I18n, key: Key, event: Event) -> Element<'a, Event> {
    button(text(i18n.text(key)).size(13))
        .on_press(event)
        .padding([8, 10])
        .style(theme::action(false))
        .into()
}

fn input<'a>(
    i18n: &I18n,
    key: Key,
    value: &str,
    on_input: impl Fn(String) -> Event + 'a,
) -> Element<'a, Event> {
    column![
        text(i18n.text(key)).size(12).style(theme::MUTED.text()),
        text_input("", value)
            .on_input(on_input)
            .padding(8)
            .style(theme::input)
    ]
    .spacing(4)
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

fn status<'a>(state: &State, i18n: &I18n) -> Element<'a, Event> {
    let mut content = column![].spacing(8);
    if !state.operation_name.is_empty() {
        content = content.push(text(state.operation_name.clone()).size(12));
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

pub(crate) fn connections_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    if state.saving {
        return text(i18n.text(Key::DeploymentSaving)).into();
    }
    let mut content = column![
        text(i18n.text(Key::DeploymentConnectionsHelp)).size(14),
        action(i18n, Key::DeploymentNewConnection, Event::NewConnection)
    ]
    .spacing(12);
    for connection in &settings.connections {
        content = content.push(
            button(text(&connection.name).size(14))
                .on_press(Event::EditConnection(connection.id.clone()))
                .padding(8)
                .style(theme::action(
                    state
                        .connection
                        .as_ref()
                        .is_some_and(|c| c.id == connection.id),
                )),
        );
    }
    content.push(status(state, i18n)).into()
}

fn connection_form<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut content = column![].spacing(12);
    if let Some(connection) = &state.connection {
        let mut protocols = row![].spacing(4);
        for (protocol, label) in [
            (Protocol::Sftp, "SFTP".to_owned()),
            (Protocol::Ftp, "FTP".to_owned()),
            (Protocol::Ftps, "FTPS".to_owned()),
            (
                Protocol::FtpsImplicit,
                i18n.text(Key::DeploymentFtpsImplicit),
            ),
        ] {
            protocols = protocols.push(
                button(text(label).size(13))
                    .on_press(Event::Protocol(protocol))
                    .padding(8)
                    .style(theme::action(protocol == connection.protocol)),
            );
        }
        content = content
            .push(protocols)
            .push(input(i18n, Key::DeploymentName, &connection.name, |v| {
                Event::Input(Field::Name, v)
            }))
            .push(
                row![
                    input(i18n, Key::DeploymentHost, &connection.host, |v| {
                        Event::Input(Field::Host, v)
                    }),
                    input(i18n, Key::DeploymentPort, &state.port, |v| Event::Input(
                        Field::Port,
                        v
                    ))
                ]
                .spacing(12),
            )
            .push(input(
                i18n,
                Key::DeploymentUsername,
                &connection.username,
                |v| Event::Input(Field::Username, v),
            ));
        if connection.protocol == Protocol::Sftp {
            let mut authentication = column![
                row![
                    action(
                        i18n,
                        Key::DeploymentPassword,
                        Event::Authentication(Authentication::Password)
                    ),
                    action(
                        i18n,
                        Key::DeploymentAgent,
                        Event::Authentication(Authentication::Agent)
                    )
                ]
                .spacing(6)
            ]
            .spacing(6);
            for key in &settings.keys {
                authentication = authentication.push(
                    button(text(&key.name).size(13))
                        .on_press(Event::Authentication(Authentication::Key(key.id.clone())))
                        .padding(8)
                        .style(theme::action(
                            connection.authentication == Authentication::Key(key.id.clone()),
                        )),
                );
            }
            authentication =
                authentication.push(action(i18n, Key::DeploymentAddIdentity, Event::NewKey));
            content = content.push(authentication).push(
                text(i18n.text(match connection.authentication {
                    Authentication::Password => Key::DeploymentPassword,
                    Authentication::Agent => Key::DeploymentAgent,
                    Authentication::Key(_) => Key::DeploymentSshIdentity,
                }))
                .size(12),
            );
        }
        if connection.authentication != Authentication::Agent {
            content = content
                .push(
                    text(i18n.text(Key::DeploymentSecretHelp))
                        .size(12)
                        .style(theme::MUTED.text()),
                )
                .push(
                    text_input(&i18n.text(Key::DeploymentPassword), &state.secret.0)
                        .secure(true)
                        .on_input(|v| Event::Password(Secret(zeroize::Zeroizing::new(v))))
                        .padding(8)
                        .style(theme::input),
                );
        }
        let save = button(text(i18n.text(Key::DeploymentSave)).size(13))
            .padding(8)
            .style(theme::action(false))
            .on_press_maybe((!state.saving).then_some(Event::SaveConnection));
        content = content.push(
            row![
                save,
                action(i18n, Key::DeploymentTest, Event::Test),
                action(i18n, Key::DeploymentDelete, Event::DeleteConnection),
                action(i18n, Key::DeploymentCancel, Event::CloseDialog)
            ]
            .spacing(6),
        );
    }
    content.push(status(state, i18n)).into()
}

pub(crate) fn keys_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut content = column![
        text(i18n.text(Key::DeploymentKeysHelp)).size(14),
        action(i18n, Key::DeploymentAddIdentity, Event::NewKey)
    ]
    .spacing(12);
    for key in &settings.keys {
        content = content.push(
            row![
                column![
                    text(&key.name).size(13),
                    text(key.path.to_string_lossy().into_owned()).size(12),
                ]
                .width(Length::Fill),
                action(
                    i18n,
                    Key::DeploymentDelete,
                    Event::DeleteKey(key.id.clone())
                ),
            ]
            .spacing(8),
        );
    }
    content.push(status(state, i18n)).into()
}

pub(crate) fn settings_dialog<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Option<Element<'a, Event>> {
    let (title, content): (Key, Element<'a, Event>) = if state.key_dialog {
        let mut fields = column![
            text(i18n.text(Key::DeploymentKeysHelp)).size(13),
            input(
                i18n,
                Key::DeploymentName,
                &state.key_name,
                |v| Event::Input(Field::KeyName, v)
            ),
            input(i18n, Key::DeploymentKeyPath, &state.key_path, |v| {
                Event::Input(Field::KeyPath, v)
            }),
            action(i18n, Key::DeploymentChooseFile, Event::PickKey),
        ]
        .spacing(12);
        if let Some(error) = &state.key_error {
            let key = if *error == Error::Configuration {
                Key::DeploymentKeyError
            } else {
                error_key(error)
            };
            fields = fields.push(text(i18n.text(key)).size(13));
        }
        fields = fields.push(
            row![
                action(i18n, Key::DeploymentSave, Event::AddKey),
                action(i18n, Key::DeploymentCancel, Event::CloseDialog),
            ]
            .spacing(8),
        );
        (Key::DeploymentAddIdentity, fields.into())
    } else if state.connection.is_some() {
        let content = if state.saving {
            text(i18n.text(Key::DeploymentSaving)).into()
        } else {
            connection_form(state, settings, i18n)
        };
        (Key::DeploymentConnection, content)
    } else {
        return None;
    };
    Some(
        column![
            text(i18n.text(title)).size(22).font(theme::semibold()),
            container(scrollable(content).style(theme::scroll)).max_height(480),
        ]
        .spacing(16)
        .into(),
    )
}

pub(crate) fn profiles_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    profiles: &'a [Profile],
    project: Option<&Path>,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut content = column![text(i18n.text(Key::DeploymentProfilesHelp)).size(14)].spacing(12);
    if project.is_some() {
        content = content.push(action(i18n, Key::DeploymentNewProfile, Event::NewProfile));
    } else {
        content = content.push(text(i18n.text(Key::DeploymentSelectProject)));
    }
    for profile in profiles {
        content = content.push(
            button(column![
                text(&profile.name).size(14),
                text(profile.project.to_string_lossy().into_owned()).size(12)
            ])
            .on_press(Event::EditProfile(profile.id.clone()))
            .padding(8)
            .style(theme::action(
                state.profile.as_ref().is_some_and(|p| p.id == profile.id),
            )),
        );
    }
    if let Some(profile) = &state.profile {
        content = content
            .push(input(i18n, Key::DeploymentName, &profile.name, |v| {
                Event::Input(Field::ProfileName, v)
            }))
            .push(text(profile.project.to_string_lossy().into_owned()).size(12))
            .push(text(i18n.text(Key::DeploymentConnection)).size(13));
        for connection in &settings.connections {
            content = content.push(
                button(text(&connection.name).size(13))
                    .on_press(Event::ProfileConnection(connection.id.clone()))
                    .padding(8)
                    .style(theme::action(connection.id == profile.connection)),
            );
        }
        content = content
            .push(input(i18n, Key::DeploymentRoot, &profile.root, |v| {
                Event::Input(Field::Root, v)
            }))
            .push(action(i18n, Key::DeploymentBrowse, Event::BrowseRoot));
        if state.browsing {
            content = content
                .push(action(
                    i18n,
                    Key::DeploymentUseRoot,
                    Event::UseRoot("/".to_owned()),
                ))
                .push(container(scrollable(tree(state, "/", 0)).style(theme::scroll)).height(220));
            if let Some(entry) = &state.selected
                && entry.directory
            {
                content = content.push(action(
                    i18n,
                    Key::DeploymentUseFolder,
                    Event::UseRoot(entry.path.clone()),
                ));
            }
        }
        content = content.push(text(i18n.text(Key::DeploymentMappings)).size(17));
        for (index, mapping) in profile.mappings.iter().enumerate() {
            content = content.push(
                row![
                    input(
                        i18n,
                        Key::DeploymentLocalFolder,
                        &mapping.local.to_string_lossy(),
                        move |v| Event::MappingLocal(index, v)
                    ),
                    input(
                        i18n,
                        Key::DeploymentRemoteFolder,
                        &mapping.remote,
                        move |v| Event::MappingRemote(index, v)
                    ),
                    action(i18n, Key::DeploymentRemove, Event::RemoveMapping(index))
                ]
                .spacing(8)
                .align_y(Alignment::End),
            );
        }
        content = content
            .push(action(i18n, Key::DeploymentAddMapping, Event::AddMapping))
            .push(
                text(i18n.text(Key::DeploymentMappingHelp))
                    .size(12)
                    .style(theme::MUTED.text()),
            )
            .push(text(i18n.text(Key::DeploymentExclusions)).size(17));
        for (index, excluded) in profile.excluded.iter().enumerate() {
            content = content.push(
                row![
                    text_input("", &excluded.to_string_lossy())
                        .on_input(move |v| Event::Exclusion(index, v))
                        .padding(8)
                        .style(theme::input),
                    action(i18n, Key::DeploymentRemove, Event::RemoveExclusion(index))
                ]
                .spacing(8),
            );
        }
        content = content
            .push(action(
                i18n,
                Key::DeploymentAddExclusion,
                Event::AddExclusion,
            ))
            .push(
                toggler(profile.default)
                    .label(i18n.text(Key::DeploymentDefault))
                    .on_toggle(Event::DefaultProfile)
                    .style(theme::toggle),
            )
            .push(
                row![
                    action(i18n, Key::DeploymentSave, Event::SaveProfile),
                    action(i18n, Key::DeploymentDelete, Event::DeleteProfile)
                ]
                .spacing(8),
            );
    }
    content.push(status(state, i18n)).into()
}

fn tree<'a>(state: &'a State, root: &str, depth: u16) -> Element<'a, Event> {
    let mut content = Column::new().spacing(2);
    if depth > 64 {
        return content.into();
    }
    if let Some(entries) = state.tree.get(root) {
        for entry in entries {
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
    for profile in profiles
        .iter()
        .filter(|p| Some(p.project.as_path()) == project)
    {
        heading = heading.push(
            button(text(&profile.name).size(13))
                .padding(6)
                .on_press_maybe((!state.busy).then(|| Event::SelectProfile(profile.id.clone())))
                .style(theme::action(
                    state
                        .active_profile(profiles, project)
                        .is_some_and(|p| p.id == profile.id),
                )),
        );
    }
    let active = state.active_profile(profiles, project);
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
            control(Icon::Settings, Key::DeploymentSettings, Event::Settings),
            control(Icon::Close, Key::DeploymentClose, Event::Toggle)
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
        container(scrollable(files).style(theme::scroll)).height(Length::Fill),
        container(status(state, i18n)).padding(10)
    ]
    .height(Length::Fill)
    .into()
}

pub(crate) fn overlay<'a, Message: Clone + 'a>(
    state: &'a State,
    i18n: &'a I18n,
    base: Element<'a, Message>,
    map: impl Fn(Event) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let Some(entry) = &state.menu else {
        return base;
    };
    context_menu::overlay(
        base,
        state.menu_point,
        iced::Size::new(260., 110.),
        move || {
            let mut actions = column![action(
                i18n,
                Key::DeploymentDownload,
                Event::Run(Operation::Download(entry.path.clone(), false))
            )]
            .spacing(6);
            if !entry.directory {
                actions = actions.push(action(
                    i18n,
                    Key::DeploymentCompare,
                    Event::CurrentFile(true),
                ));
            }
            Element::from(actions).map(map)
        },
        map(Event::DismissMenu),
    )
}

#[cfg(test)]
mod tests;
