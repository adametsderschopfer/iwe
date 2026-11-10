//! Present the bundled module catalog and its shared controls.

use crate::{
    app::{App, Message, modules::Event},
    i18n::Key,
    modules::{MODULES, ModuleId, Origin},
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text, toggler},
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
