//! Present the module catalog and its shared controls.

use crate::{
    app::{App, Message, modules::Event},
    domain::modules::{Permission, Placement},
    i18n::Key,
    modules::{MODULES, ModuleId, Origin},
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, toggler},
};

pub(crate) fn title(id: ModuleId) -> Key {
    match id {
        ModuleId::Git => Key::Git,
        ModuleId::Pipelines => Key::Pipelines,
        ModuleId::Terminal => Key::Terminal,
        ModuleId::Deployment => Key::Deployment,
    }
}

pub(crate) fn panel(app: &App, id: ModuleId) -> Element<'_, Message> {
    match id {
        ModuleId::Git => crate::ui::screens::git::view(app),
        ModuleId::Pipelines => crate::ui::screens::pipelines::view(app),
        ModuleId::Deployment => crate::modules::deployment::panel(
            &app.modules.deployment,
            &app.workspace.deployment,
            app.deployment_profiles(),
            app.workspace.directory().map(std::path::PathBuf::as_path),
            &app.i18n,
        )
        .map(crate::app::modules::deployment_message),
        ModuleId::Terminal => crate::modules::terminal::view(
            &app.modules.terminal,
            &app.i18n,
            app.workspace.preferences.code_size,
            app.auth_dialog.is_none()
                && app.space_draft.is_none()
                && app.project_add.is_none()
                && app.line_navigation.is_none()
                && !app.modules.git.git_actions.has_dialog(),
        )
        .map(|event| Message::Module(Event::Terminal(event))),
    }
}

fn external(id: &crate::domain::modules::Manifest) -> Message {
    Message::Module(Event::External(
        crate::app::modules::external::Event::Remove(id.id.clone()),
    ))
}

fn set_external_enabled(id: &str, enabled: bool) -> Message {
    Message::Module(Event::External(
        crate::app::modules::external::Event::SetEnabled(id.to_owned(), enabled),
    ))
}

pub(crate) fn permission_key(permission: Permission) -> Key {
    match permission {
        Permission::FilesystemRead => Key::PermissionFilesystemRead,
        Permission::FilesystemWrite => Key::PermissionFilesystemWrite,
        Permission::Network => Key::PermissionNetwork,
        Permission::Process => Key::PermissionProcess,
    }
}

pub(crate) fn placement_key(placement: Placement) -> Key {
    match placement {
        Placement::Header => Key::PlacementHeader,
        Placement::Top => Key::PlacementTop,
        Placement::Left => Key::PlacementLeft,
        Placement::Right => Key::PlacementRight,
        Placement::Bottom => Key::PlacementBottom,
    }
}

pub(crate) fn settings(app: &App) -> Element<'_, Message> {
    let mut list = column![
        text(app.i18n.text(Key::ModulesDescription))
            .size(13)
            .style(theme::MUTED.text()),
    ]
    .spacing(0);
    for module in MODULES {
        list = list.push(
            container(column![
                row![
                    text(app.i18n.text(title(module.id)))
                        .size(14)
                        .width(Length::Fill),
                    container(
                        text(app.i18n.text(match module.origin {
                            Origin::System => Key::SystemModule,
                        }))
                        .size(12)
                    )
                    .padding([3, 8])
                    .style(theme::badge),
                    container(
                        toggler(app.module_enabled(module.id))
                            .style(theme::toggle)
                            .on_toggle(move |enabled| Message::Module(Event::SetEnabled(
                                module.id, enabled
                            )))
                    )
                    .id(format!("module-{}-enabled", module.id.key())),
                ]
                .spacing(16)
                .align_y(Alignment::Center)
                .padding([14, 0]),
                container(iced::widget::space())
                    .height(1)
                    .width(Length::Fill)
                    .style(theme::separator),
            ])
            .id(format!("module-{}", module.id.key()))
            .width(Length::Fill),
        );
    }
    list.into()
}

/// External module section of the Modules settings tab.
pub(crate) fn external_settings(app: &App) -> Element<'_, Message> {
    let external_state = &app.modules.external;
    let mut list = column![].spacing(8);
    list = list.push(
        row![
            text(app.i18n.text(Key::ExternalModules))
                .size(14)
                .font(theme::semibold())
                .width(Length::Fill),
            button(
                text(if external_state.installing {
                    app.i18n.text(Key::InstallingModule)
                } else {
                    app.i18n.text(Key::InstallModule)
                })
                .size(13),
            )
            .on_press_maybe(
                (!external_state.installing && external_state.confirm.is_none()).then_some(
                    Message::Module(Event::External(crate::app::modules::external::Event::Pick,)),
                ),
            )
            .padding([8, 14])
            .style(theme::primary),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );
    if let Some(pending) = &external_state.confirm {
        list = list.push(confirm_card(app, pending));
    }
    let installed = &external_state.registry.modules;
    if installed.is_empty() {
        list = list.push(
            container(
                text(app.i18n.text(Key::NoExternalModules))
                    .size(13)
                    .style(theme::MUTED.text()),
            )
            .padding([8, 0]),
        );
    }
    for module in installed {
        let id = module.manifest.id.clone();
        let name = module
            .manifest
            .name
            .resolve(app.i18n.language().code())
            .to_owned();
        let description = module
            .manifest
            .description
            .resolve(app.i18n.language().code())
            .to_owned();
        let enabled = app.external_enabled(&id);
        let mut permissions = row![].spacing(6);
        for permission in &module.manifest.permissions {
            permissions = permissions.push(
                container(text(app.i18n.text(permission_key(*permission))).size(11))
                    .padding([2, 6])
                    .style(theme::badge),
            );
        }
        list = list.push(
            container(column![
                row![
                    column![
                        row![
                            text(name).size(14),
                            container(text(app.i18n.text(Key::ExternalModule)).size(12))
                                .padding([3, 8])
                                .style(theme::badge),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(description).size(12).style(theme::MUTED.text()),
                        row![
                            text(format!(
                                "{} · {}",
                                module.manifest.version,
                                app.i18n.text(placement_key(module.manifest.placement)),
                            ))
                            .size(12)
                            .style(theme::FAINT.text()),
                            permissions,
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    button(text(app.i18n.text(Key::RemoveModule)).size(13))
                        .on_press(external(&module.manifest))
                        .padding([8, 14])
                        .style(theme::secondary),
                    container(
                        toggler(enabled)
                            .style(theme::toggle)
                            .on_toggle(move |value| set_external_enabled(&id, value)),
                    )
                    .id(format!("external-module-{}-enabled", module.manifest.id)),
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                container(iced::widget::space())
                    .height(1)
                    .width(Length::Fill)
                    .style(theme::separator),
            ])
            .id(format!("external-module-{}", module.manifest.id))
            .width(Length::Fill),
        );
    }
    list.into()
}

fn confirm_card<'a>(
    app: &'a App,
    pending: &'a crate::app::modules::external::Pending,
) -> Element<'a, Message> {
    let mut args = crate::i18n::FluentArgs::new();
    args.set(
        "name",
        pending.manifest.name.resolve(app.i18n.language().code()),
    );
    let mut permissions = column![].spacing(4);
    for permission in &pending.manifest.permissions {
        permissions = permissions.push(text(app.i18n.text(permission_key(*permission))).size(12));
    }
    if pending.manifest.permissions.is_empty() {
        permissions = permissions.push(
            text(app.i18n.text(Key::ModulePermissions))
                .size(12)
                .style(theme::FAINT.text()),
        );
    }
    container(
        column![
            text(app.i18n.format(Key::ConfirmInstallTitle, &args))
                .size(14)
                .font(theme::semibold()),
            text(app.i18n.text(Key::ConfirmInstallDescription))
                .size(12)
                .style(theme::MUTED.text()),
            permissions,
            row![
                button(text(app.i18n.text(Key::InstallModule)).size(13))
                    .on_press(Message::Module(Event::External(
                        crate::app::modules::external::Event::Confirm,
                    )))
                    .padding([8, 14])
                    .style(theme::primary),
                button(text(app.i18n.text(Key::Cancel)).size(13))
                    .on_press(Message::Module(Event::External(
                        crate::app::modules::external::Event::Cancel,
                    )))
                    .padding([8, 14])
                    .style(theme::secondary),
            ]
            .spacing(8),
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Length::Fill)
    .style(theme::surface)
    .into()
}
