//! Present compact server lists, aligned forms, and focused settings dialogs.

use super::{error_key, status, style, tree};
use crate::{
    domain::deployment::{Authentication, Connections, Error, Profile, Protocol, safe_text},
    i18n::{I18n, Key},
    modules::deployment::{
        state::{Event, Field, NameAction, ProfileTab, State},
        transport::Secret,
    },
    ui::{
        icons::{Icon, icon},
        theme, tooltip,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, button, column, container, pick_list, row, scrollable, space, text, text_input,
        toggler,
    },
};
use std::path::Path;

const LABEL_WIDTH: f32 = 128.;
const LIST_WIDTH: f32 = 176.;

#[derive(Clone, PartialEq)]
struct Choice<T> {
    value: T,
    label: String,
}
impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn action<'a>(i18n: &I18n, key: Key, event: Event) -> Element<'a, Event> {
    button(text(i18n.text(key)).size(13))
        .on_press(event)
        .padding([7, 12])
        .style(style::secondary)
        .into()
}

fn select<'a, T: Clone + PartialEq + 'a>(
    options: Vec<(T, String)>,
    selected: Option<&T>,
    on_select: impl Fn(T) -> Event + 'a,
) -> Element<'a, Event> {
    let options: Vec<_> = options
        .into_iter()
        .map(|(value, label)| Choice { value, label })
        .collect();
    let selected = options
        .iter()
        .find(|option| Some(&option.value) == selected)
        .cloned();
    pick_list(options, selected, move |choice| on_select(choice.value))
        .text_size(13)
        .padding([6, 9])
        .width(Length::Fill)
        .style(|theme, status| {
            let mut style = theme::select(theme, status);
            style.border.radius = 4.into();
            style
        })
        .menu_style(theme::select_menu)
        .into()
}

fn feedback<'a>(state: &'a State, i18n: &'a I18n) -> Element<'a, Event> {
    if let Some(error @ (Error::Configuration | Error::Mapping)) = &state.error {
        text(i18n.text(error_key(error))).size(13).into()
    } else if state.busy || state.error.is_some() || state.status.is_some() {
        status(state, i18n)
    } else {
        column![].into()
    }
}

fn test_controls<'a>(
    state: &'a State,
    i18n: &I18n,
    event: Event,
    connection: Option<&crate::domain::deployment::Connection>,
) -> Element<'a, Event> {
    let relevant = state.operation_label == Some(Key::DeploymentTest)
        && state.test_connection.is_some()
        && state.test_connection.as_ref() == connection;
    let outcome = if !relevant {
        None
    } else if state.busy {
        Some((Key::DeploymentChecking, theme::MUTED))
    } else {
        state.test_result.as_ref().map(|result| match result {
            Ok(()) => (Key::DeploymentConnected, theme::GREEN),
            Err(error) => (error_key(error), theme::RED),
        })
    };
    let mut controls = column![].spacing(8).width(Length::Fill);
    if let Some((key, color)) = outcome {
        controls = controls.push(
            container(text(i18n.text(key)).size(13).style(color.text()))
                .id("deployment-test-status")
                .width(Length::Fill),
        );
    }
    if relevant
        && !state.busy
        && let Some(Error::HostUnknown(fingerprint)) = &state.error
    {
        controls = controls
            .push(text(fingerprint).size(12).style(theme::MUTED.text()))
            .push(
                row![
                    action(i18n, Key::DeploymentTrust, Event::TrustHost),
                    action(i18n, Key::DeploymentCancel, Event::Cancel),
                ]
                .spacing(8),
            );
    }
    let mut buttons = row![
        container(
            button(text(i18n.text(Key::DeploymentTest)).size(13))
                .padding([7, 12])
                .style(style::secondary)
                .on_press_maybe((!state.busy).then_some(event))
        )
        .id("deployment-test-button"),
    ]
    .spacing(8);
    if relevant && state.busy {
        buttons = buttons.push(action(i18n, Key::DeploymentCancel, Event::Cancel));
    }
    controls = controls.push(buttons);
    row![space().width(LABEL_WIDTH), controls]
        .spacing(12)
        .into()
}

fn protocols(i18n: &I18n) -> Vec<(Protocol, String)> {
    vec![
        (Protocol::Sftp, "SFTP".into()),
        (Protocol::Ftp, "FTP".into()),
        (Protocol::Ftps, "FTPS".into()),
        (
            Protocol::FtpsImplicit,
            i18n.text(Key::DeploymentFtpsImplicit),
        ),
    ]
}

fn field<'a>(value: &str, on_input: impl Fn(String) -> Event + 'a) -> Element<'a, Event> {
    text_input("", value)
        .on_input(on_input)
        .size(13)
        .padding([6, 9])
        .style(|theme, status| {
            let mut style = theme::input(theme, status);
            style.border.radius = 4.into();
            style
        })
        .into()
}

fn form_row<'a>(
    i18n: &I18n,
    key: Key,
    control: impl Into<Element<'a, Event>>,
) -> Element<'a, Event> {
    row![
        text(i18n.text(key)).size(13).width(LABEL_WIDTH),
        control.into()
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn rule<'a>() -> Element<'a, Event> {
    container(space())
        .height(1)
        .width(Length::Fill)
        .style(theme::separator)
        .into()
}

fn tool<'a>(i18n: &I18n, symbol: Icon, key: Key, event: Option<Event>) -> Element<'a, Event> {
    tooltip::view(
        button(icon(symbol, theme::MUTED))
            .on_press_maybe(event)
            .padding(5)
            .style(theme::action(false)),
        text(i18n.text(key)).size(12),
        tooltip::Position::Bottom,
    )
    .into()
}

fn list_row<'a>(
    name: &'a str,
    selected: bool,
    event: Option<Event>,
    default: bool,
) -> Element<'a, Event> {
    let mut line = row![
        icon(Icon::Server, theme::MUTED),
        text(name)
            .size(13)
            .width(Length::Fill)
            .wrapping(text::Wrapping::None)
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if default {
        line = line.push(icon(Icon::Check, theme::BLUE));
    }
    button(line)
        .on_press_maybe(event)
        .padding([7, 8])
        .width(Length::Fill)
        .style(move |theme: &iced::Theme, status| {
            let mut style = theme::action(false)(theme, status);
            if selected {
                style.background = Some(theme::BLUE.resolve(theme).scale_alpha(0.18).into());
                style.text_color = theme::TEXT.resolve(theme);
            }
            style.border.radius = 4.into();
            style
        })
        .into()
}

fn editor<'a>(
    tools: Element<'a, Event>,
    list: Column<'a, Event>,
    detail: Element<'a, Event>,
) -> Element<'a, Event> {
    let list = container(column![
        container(tools).padding([4, 6]),
        rule(),
        scrollable(list.spacing(2).padding(6))
            .style(theme::scroll)
            .height(Length::Fill)
    ])
    .width(LIST_WIDTH)
    .height(Length::Fill)
    .style(theme::sidebar);
    row![
        list,
        container(space())
            .width(1)
            .height(Length::Fill)
            .style(theme::separator),
        detail
    ]
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn footer<'a>(i18n: &I18n, save: Event, dirty: bool, saving: bool) -> Element<'a, Event> {
    row![
        space().width(Length::Fill),
        button(
            text(i18n.text(if saving {
                Key::DeploymentSaving
            } else {
                Key::DeploymentSave
            }))
            .size(13)
        )
        .padding([7, 16])
        .style(style::primary)
        .on_press_maybe((dirty && !saving).then_some(save))
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn connections_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let enabled = !state.saving && !state.busy;
    let selected = state.connection.as_ref();
    let tools = row![
        tool(
            i18n,
            Icon::Plus,
            Key::DeploymentNewConnection,
            enabled.then_some(Event::BeginName(NameAction::NewConnection))
        ),
        tool(
            i18n,
            Icon::Minus,
            Key::DeploymentDelete,
            (enabled && selected.is_some()).then_some(Event::DeleteConnection)
        ),
        tool(
            i18n,
            Icon::Duplicate,
            Key::DeploymentDuplicate,
            (enabled && selected.is_some())
                .then_some(Event::BeginName(NameAction::DuplicateConnection))
        ),
        tool(
            i18n,
            Icon::Rename,
            Key::DeploymentRename,
            (enabled && selected.is_some())
                .then_some(Event::BeginName(NameAction::RenameConnection))
        ),
    ]
    .spacing(2)
    .into();
    let mut list = column![];
    for connection in &settings.connections {
        let draft = selected.filter(|c| c.id == connection.id);
        list = list.push(list_row(
            draft.map_or(&connection.name, |c| &c.name),
            draft.is_some(),
            enabled.then(|| Event::EditConnection(connection.id.clone())),
            false,
        ));
    }
    if let Some(draft) =
        selected.filter(|c| !settings.connections.iter().any(|saved| saved.id == c.id))
    {
        list = list.push(list_row(&draft.name, true, None, false));
    }
    let header = row![
        text(i18n.text(Key::DeploymentConnectionsHelp))
            .size(12)
            .style(theme::MUTED.text())
            .width(Length::Fill),
        action(i18n, Key::DeploymentKeys, Event::OpenKeys)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let body = if selected.is_some() {
        if state.saving {
            text(i18n.text(Key::DeploymentSaving)).into()
        } else {
            connection_form(state, settings, i18n)
        }
    } else {
        column![
            text(i18n.text(Key::DeploymentChooseConnection))
                .size(14)
                .style(theme::MUTED.text()),
            action(
                i18n,
                Key::DeploymentNewConnection,
                Event::BeginName(NameAction::NewConnection)
            )
        ]
        .spacing(12)
        .into()
    };
    let detail = container(
        column![
            header,
            rule(),
            scrollable(body).style(theme::scroll).height(Length::Fill),
            footer(
                i18n,
                Event::SaveConnection,
                state.connection_dirty(settings),
                state.saving
            )
        ]
        .spacing(16),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
    editor(tools, list, detail)
}

pub(super) fn connection_form<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let Some(connection) = &state.connection else {
        return column![].into();
    };
    let mut form = column![
        form_row(
            i18n,
            Key::DeploymentName,
            field(&connection.name, |v| Event::Input(Field::Name, v))
        ),
        form_row(
            i18n,
            Key::DeploymentProtocol,
            select(protocols(i18n), Some(&connection.protocol), Event::Protocol)
        ),
        form_row(
            i18n,
            Key::DeploymentHost,
            row![
                field(&connection.host, |v| Event::Input(Field::Host, v)),
                text(i18n.text(Key::DeploymentPort)).size(13),
                container(field(&state.port, |v| Event::Input(Field::Port, v))).width(70)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        ),
        form_row(
            i18n,
            Key::DeploymentUsername,
            field(&connection.username, |v| Event::Input(Field::Username, v))
        ),
    ]
    .spacing(12);
    if connection.protocol == Protocol::Sftp {
        let mode = match connection.authentication {
            Authentication::Password => 0,
            Authentication::Agent => 1,
            Authentication::Key(_) => 2,
        };
        form = form.push(form_row(
            i18n,
            Key::DeploymentAuthentication,
            select(
                vec![
                    (0, i18n.text(Key::DeploymentPassword)),
                    (1, i18n.text(Key::DeploymentAgent)),
                    (2, i18n.text(Key::DeploymentSshIdentity)),
                ],
                Some(&mode),
                |mode| {
                    Event::Authentication(match mode {
                        1 => Authentication::Agent,
                        2 => Authentication::Key(String::new()),
                        _ => Authentication::Password,
                    })
                },
            ),
        ));
        if let Authentication::Key(id) = &connection.authentication {
            form = form.push(form_row(
                i18n,
                Key::DeploymentSshIdentity,
                row![
                    select(
                        settings
                            .keys
                            .iter()
                            .map(|k| (k.id.clone(), k.name.clone()))
                            .collect(),
                        Some(id),
                        |id| Event::Authentication(Authentication::Key(id))
                    ),
                    tool(
                        i18n,
                        Icon::Plus,
                        Key::DeploymentAddIdentity,
                        Some(Event::NewKey)
                    ),
                    tool(
                        i18n,
                        Icon::Settings,
                        Key::DeploymentKeys,
                        Some(Event::OpenKeys)
                    ),
                ]
                .spacing(6),
            ));
        }
    }
    if connection.authentication != Authentication::Agent {
        let label = if matches!(connection.authentication, Authentication::Key(_)) {
            Key::DeploymentPassphrase
        } else {
            Key::DeploymentPassword
        };
        form = form
            .push(form_row(
                i18n,
                label,
                text_input("", &state.secret.0)
                    .secure(true)
                    .size(13)
                    .padding([6, 9])
                    .on_input(|v| Event::Password(Secret(zeroize::Zeroizing::new(v))))
                    .style(theme::input),
            ))
            .push(
                row![
                    space().width(LABEL_WIDTH),
                    text(i18n.text(Key::DeploymentSecretHelp))
                        .size(12)
                        .style(theme::MUTED.text())
                        .width(Length::Fill)
                ]
                .spacing(12),
            );
    }
    if state.operation_label.is_none() && state.error.is_some() {
        form = form.push(feedback(state, i18n));
    }
    form.push(test_controls(state, i18n, Event::Test, Some(connection)))
        .into()
}

pub(crate) fn profiles_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    profiles: &'a [Profile],
    project: Option<&Path>,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let selected = state.profile.as_ref();
    let tools = row![
        tool(
            i18n,
            Icon::Plus,
            Key::DeploymentNewProfile,
            project.map(|_| Event::BeginName(NameAction::NewProfile))
        ),
        tool(
            i18n,
            Icon::Minus,
            Key::DeploymentDelete,
            (selected.is_some() && !state.busy).then_some(Event::DeleteProfile)
        ),
        tool(
            i18n,
            Icon::Duplicate,
            Key::DeploymentDuplicate,
            selected.map(|_| Event::BeginName(NameAction::DuplicateProfile))
        ),
        tool(
            i18n,
            Icon::Rename,
            Key::DeploymentRename,
            selected.map(|_| Event::BeginName(NameAction::RenameProfile))
        ),
    ]
    .spacing(2)
    .into();
    let mut list = column![];
    for profile in profiles {
        let draft = selected.filter(|p| p.id == profile.id);
        list = list.push(list_row(
            draft.map_or(&profile.name, |p| &p.name),
            draft.is_some(),
            Some(Event::EditProfile(profile.id.clone())),
            draft.unwrap_or(profile).default,
        ));
    }
    if let Some(draft) = selected.filter(|p| !profiles.iter().any(|saved| saved.id == p.id)) {
        list = list.push(list_row(&draft.name, true, None, draft.default));
    }
    let body = if let Some(profile) = selected {
        profile_form(state, settings, profile, i18n)
    } else {
        column![
            text(i18n.text(if project.is_some() {
                Key::DeploymentChooseProfile
            } else {
                Key::DeploymentSelectProject
            }))
            .size(14)
            .style(theme::MUTED.text())
        ]
        .into()
    };
    let detail = container(
        column![
            text(i18n.text(Key::DeploymentProfilesHelp))
                .size(12)
                .width(Length::Fill)
                .style(theme::MUTED.text()),
            profile_tabs(state.profile_tab, i18n),
            scrollable(body).style(theme::scroll).height(Length::Fill),
            footer(
                i18n,
                Event::SaveProfile,
                state.profile_dirty(profiles),
                false
            )
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
    editor(tools, list, detail)
}

pub(super) fn profile_tabs<'a>(active: ProfileTab, i18n: &I18n) -> Element<'a, Event> {
    let mut tabs = row![];
    for (tab, key, name) in [
        (
            ProfileTab::Connection,
            Key::DeploymentConnection,
            "connection",
        ),
        (ProfileTab::Mappings, Key::DeploymentMappings, "mappings"),
        (
            ProfileTab::Exclusions,
            Key::DeploymentExclusions,
            "exclusions",
        ),
    ] {
        let selected = active == tab;
        let label = container(text(i18n.text(key)).size(13).wrapping(text::Wrapping::None))
            .id(format!("deployment-profile-tab-label-{name}"));
        let tab = button(
            column![
                container(label).padding([9, 12]),
                container(space())
                    .id(format!("deployment-profile-tab-indicator-{name}"))
                    .height(2)
                    .width(Length::Fill)
                    .style(move |theme: &iced::Theme| container::Style {
                        background: selected.then(|| theme::BLUE.resolve(theme).into()),
                        ..Default::default()
                    }),
            ]
            .width(Length::Shrink),
        )
        .padding(0)
        .on_press(Event::ProfileTab(tab))
        .style(theme::action(false));
        tabs = tabs.push(container(tab).id(format!("deployment-profile-tab-{name}")));
    }
    column![
        scrollable(tabs)
            .id("deployment-profile-tabs")
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::hidden(),
            ))
            .width(Length::Fill)
            .style(theme::scroll),
        rule(),
    ]
    .spacing(0)
    .width(Length::Fill)
    .into()
}

fn profile_form<'a>(
    state: &'a State,
    settings: &'a Connections,
    profile: &'a Profile,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut form = column![].spacing(12);
    match state.profile_tab {
        ProfileTab::Connection => {
            form = form
                .push(form_row(
                    i18n,
                    Key::DeploymentName,
                    field(&profile.name, |v| Event::Input(Field::ProfileName, v)),
                ))
                .push(form_row(
                    i18n,
                    Key::DeploymentProject,
                    text(profile.project.to_string_lossy().into_owned()).size(13),
                ))
                .push(form_row(
                    i18n,
                    Key::DeploymentConnection,
                    row![
                        select(
                            settings
                                .connections
                                .iter()
                                .map(|c| (c.id.clone(), c.name.clone()))
                                .collect(),
                            Some(&profile.connection),
                            Event::ProfileConnection
                        ),
                        tool(
                            i18n,
                            Icon::Settings,
                            Key::DeploymentConnections,
                            Some(Event::ShowConnections)
                        )
                    ]
                    .spacing(6),
                ))
                .push(form_row(
                    i18n,
                    Key::DeploymentRoot,
                    row![
                        field(&profile.root, |v| Event::Input(Field::Root, v)),
                        tool(
                            i18n,
                            Icon::Folder,
                            Key::DeploymentBrowse,
                            (!state.busy).then_some(Event::BrowseRoot)
                        )
                    ]
                    .spacing(6),
                ))
                .push(test_controls(
                    state,
                    i18n,
                    Event::TestProfile,
                    settings
                        .connections
                        .iter()
                        .find(|c| c.id == profile.connection),
                ))
                .push(
                    toggler(profile.default)
                        .label(i18n.text(Key::DeploymentDefault))
                        .on_toggle(Event::DefaultProfile)
                        .style(theme::toggle),
                );
        }
        ProfileTab::Mappings => {
            form = form
                .push(form_row(
                    i18n,
                    Key::DeploymentProject,
                    text(profile.project.to_string_lossy().into_owned()).size(12),
                ))
                .push(
                    text(i18n.text(Key::DeploymentMappingHelp))
                        .size(12)
                        .style(theme::MUTED.text()),
                );
            for (index, mapping) in profile.mappings.iter().enumerate() {
                form = form.push(
                    container(
                        column![
                            row![
                                text((index + 1).to_string())
                                    .size(12)
                                    .style(theme::MUTED.text()),
                                space().width(Length::Fill),
                                tool(
                                    i18n,
                                    Icon::Close,
                                    Key::DeploymentRemove,
                                    Some(Event::RemoveMapping(index))
                                )
                            ]
                            .align_y(Alignment::Center),
                            form_row(
                                i18n,
                                Key::DeploymentLocalFolder,
                                row![
                                    field(
                                        &profile.project.join(&mapping.local).to_string_lossy(),
                                        move |v| { Event::MappingLocal(index, v) }
                                    ),
                                    tool(
                                        i18n,
                                        Icon::Folder,
                                        Key::DeploymentBrowse,
                                        Some(Event::PickLocalMapping(index))
                                    )
                                ]
                                .spacing(8)
                            ),
                            form_row(
                                i18n,
                                Key::DeploymentRemoteFolder,
                                row![
                                    field(&mapping.remote, move |v| Event::MappingRemote(index, v)),
                                    tool(
                                        i18n,
                                        Icon::Folder,
                                        Key::DeploymentBrowse,
                                        (!state.busy).then_some(Event::BrowseMapping(index))
                                    )
                                ]
                                .spacing(8)
                            ),
                        ]
                        .spacing(8),
                    )
                    .padding(12)
                    .width(Length::Fill)
                    .style(theme::sidebar),
                );
            }
            form = form.push(action(i18n, Key::DeploymentAddMapping, Event::AddMapping));
        }
        ProfileTab::Exclusions => {
            form = form.push(
                text(i18n.text(Key::DeploymentExclusionsHelp))
                    .size(12)
                    .style(theme::MUTED.text()),
            );
            for (index, path) in profile.excluded.iter().enumerate() {
                form = form.push(
                    row![
                        field(&path.to_string_lossy(), move |v| Event::Exclusion(index, v)),
                        tool(
                            i18n,
                            Icon::Close,
                            Key::DeploymentRemove,
                            Some(Event::RemoveExclusion(index))
                        )
                    ]
                    .spacing(8),
                );
            }
            form = form.push(action(
                i18n,
                Key::DeploymentAddExclusion,
                Event::AddExclusion,
            ));
        }
    }
    if state.profile_tab != ProfileTab::Connection
        || matches!(state.error, Some(Error::Configuration | Error::Mapping))
    {
        form = form.push(feedback(state, i18n));
    }
    form.into()
}

pub(super) fn keys_view<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Element<'a, Event> {
    let mut list = column![
        text(i18n.text(Key::DeploymentKeysHelp))
            .size(12)
            .style(theme::MUTED.text())
    ]
    .spacing(12);
    for key in &settings.keys {
        list = list.push(
            row![
                column![
                    text(&key.name).size(13),
                    text(key.path.to_string_lossy().into_owned())
                        .size(12)
                        .style(theme::MUTED.text())
                ]
                .width(Length::Fill),
                tool(
                    i18n,
                    Icon::Rename,
                    Key::DeploymentEdit,
                    Some(Event::EditKey(key.id.clone()))
                ),
                tool(
                    i18n,
                    Icon::Close,
                    Key::DeploymentDelete,
                    Some(Event::DeleteKey(key.id.clone()))
                )
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    list.push(action(i18n, Key::DeploymentAddIdentity, Event::NewKey))
        .push(feedback(state, i18n))
        .push(row![
            space().width(Length::Fill),
            action(i18n, Key::DeploymentClose, Event::CloseDialog)
        ])
        .into()
}

pub(crate) fn settings_dialog<'a>(
    state: &'a State,
    settings: &'a Connections,
    i18n: &'a I18n,
) -> Option<Element<'a, Event>> {
    let (title, content): (Key, Element<'a, Event>) = if state.pending.is_some() {
        (
            Key::DeploymentUnsaved,
            column![
                text(i18n.text(Key::DeploymentDiscardHelp)).size(13),
                row![
                    space().width(Length::Fill),
                    action(i18n, Key::DeploymentCancel, Event::CloseDialog),
                    action(i18n, Key::DeploymentDiscard, Event::DiscardChanges)
                ]
                .spacing(8)
            ]
            .spacing(16)
            .into(),
        )
    } else if state.browsing {
        let selected = state
            .selected
            .as_ref()
            .filter(|entry| entry.directory && !entry.symlink);
        let mut browser = column![
            container(scrollable(tree(state, state.browsing_root(), 0)).style(theme::scroll))
                .id("deployment-folder-tree")
                .height(320)
                .width(Length::Fill),
        ]
        .spacing(12);
        if state.busy {
            browser = browser.push(text(i18n.text(Key::Loading)).size(12));
        }
        if let Some(error) = &state.error {
            browser = browser.push(
                text(i18n.text(super::error_key(error)))
                    .size(12)
                    .style(theme::RED.text()),
            );
        }
        browser = browser.push(
            row![
                action(i18n, Key::DeploymentCancel, Event::StopBrowsing),
                space().width(Length::Fill),
                action(
                    i18n,
                    Key::DeploymentUseRoot,
                    Event::UseRoot(state.browsing_root().to_owned())
                ),
                button(text(i18n.text(Key::DeploymentUseFolder)).size(13))
                    .style(style::primary)
                    .padding([8, 12])
                    .on_press_maybe(selected.map(|entry| Event::UseRoot(entry.path.clone()))),
            ]
            .spacing(8),
        );
        (Key::DeploymentBrowse, browser.into())
    } else if let Some(name_action) = state.naming {
        let mut content = column![form_row(
            i18n,
            Key::DeploymentName,
            field(&state.name_value, |v| Event::Input(Field::DraftName, v))
        )]
        .spacing(12);
        if name_action == NameAction::NewConnection {
            content = content.push(form_row(
                i18n,
                Key::DeploymentProtocol,
                select(
                    protocols(i18n),
                    Some(&state.new_protocol),
                    Event::NewProtocol,
                ),
            ));
        }
        if let Some(error) = &state.error {
            content = content.push(
                text(i18n.text(super::error_key(error)))
                    .size(12)
                    .style(theme::RED.text()),
            );
        }
        content = content.push(
            row![
                space().width(Length::Fill),
                action(i18n, Key::DeploymentCancel, Event::CloseDialog),
                button(text(i18n.text(Key::DeploymentContinue)).size(13))
                    .padding([8, 12])
                    .style(style::primary)
                    .on_press_maybe(
                        (!state.name_value.trim().is_empty() && safe_text(&state.name_value))
                            .then_some(Event::ConfirmName)
                    )
            ]
            .spacing(8),
        );
        let title = match name_action {
            NameAction::NewConnection => Key::DeploymentNewConnection,
            NameAction::NewProfile => Key::DeploymentNewProfile,
            NameAction::DuplicateConnection | NameAction::DuplicateProfile => {
                Key::DeploymentDuplicate
            }
            _ => Key::DeploymentRename,
        };
        (title, content.into())
    } else if state.key_dialog {
        let mut content = column![
            text(i18n.text(Key::DeploymentKeysHelp))
                .size(12)
                .style(theme::MUTED.text()),
            form_row(
                i18n,
                Key::DeploymentName,
                field(&state.key_name, |v| Event::Input(Field::KeyName, v))
            ),
            form_row(
                i18n,
                Key::DeploymentKeyPath,
                row![
                    field(&state.key_path, |v| Event::Input(Field::KeyPath, v)),
                    tool(
                        i18n,
                        Icon::Folder,
                        Key::DeploymentChooseFile,
                        Some(Event::PickKey)
                    )
                ]
                .spacing(6)
            ),
        ]
        .spacing(12);
        if let Some(error) = &state.key_error {
            content = content.push(
                text(i18n.text(if *error == Error::Configuration {
                    Key::DeploymentKeyError
                } else {
                    error_key(error)
                }))
                .size(13),
            );
        }
        content = content.push(
            row![
                space().width(Length::Fill),
                action(i18n, Key::DeploymentCancel, Event::CloseDialog),
                action(i18n, Key::DeploymentSave, Event::AddKey)
            ]
            .spacing(8),
        );
        (
            if state.key_editing.is_some() {
                Key::DeploymentSshIdentity
            } else {
                Key::DeploymentAddIdentity
            },
            content.into(),
        )
    } else if state.keys_open {
        (Key::DeploymentKeys, keys_view(state, settings, i18n))
    } else {
        return None;
    };
    Some(
        column![
            text(i18n.text(title)).size(19).font(theme::semibold()),
            container(scrollable(content).style(theme::scroll)).max_height(480)
        ]
        .spacing(20)
        .into(),
    )
}

pub(super) fn profile_selector<'a>(
    profiles: &'a [Profile],
    selected: Option<&Profile>,
    project: Option<&Path>,
) -> Element<'a, Event> {
    select(
        profiles
            .iter()
            .filter(|p| Some(p.project.as_path()) == project)
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect(),
        selected.map(|p| &p.id),
        Event::SelectProfile,
    )
}
