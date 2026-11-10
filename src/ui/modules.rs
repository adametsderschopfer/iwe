//! Present the bundled module catalog and its shared controls.

use crate::{
    app::{App, Message, modules::Event},
    i18n::Key,
    modules::{MODULES, ModuleId, Origin},
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

pub(crate) fn title(id: ModuleId) -> Key {
    match id {
        ModuleId::Terminal => Key::Terminal,
        ModuleId::Deployment => Key::Deployment,
    }
}

pub(crate) fn panel(app: &App, id: ModuleId) -> Element<'_, Message> {
    match id {
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
                && !app.git_actions.has_dialog(),
        )
        .map(|event| Message::Module(Event::Terminal(event))),
    }
}

pub(crate) fn settings(app: &App) -> Element<'_, Message> {
    let mut cards = column![
        text(app.i18n.text(Key::ModulesDescription))
            .size(14)
            .style(theme::MUTED.text()),
    ]
    .spacing(20);
    for module in MODULES {
        let id = module.id;
        let heading = row![
            text(app.i18n.text(title(id)))
                .size(17)
                .font(theme::semibold()),
            container(
                text(app.i18n.text(match module.origin {
                    Origin::System => Key::SystemModule,
                }))
                .size(12)
            )
            .padding([4, 8])
            .style(theme::badge),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        cards = cards.push(
            container(
                column![
                    heading,
                    text(app.i18n.text(Key::ModuleEnabled))
                        .size(12)
                        .style(theme::MUTED.text()),
                ]
                .spacing(12),
            )
            .id(format!("module-{}", id.key()))
            .padding(20)
            .width(Length::Fill)
            .style(theme::sidebar),
        );
    }
    cards.into()
}
