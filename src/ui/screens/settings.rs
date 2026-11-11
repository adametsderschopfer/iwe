use super::workspace::divider;
use crate::{
    app::{App, Message, Panel, SettingsTab},
    domain::{shortcuts::Command, workspace::Appearance},
    i18n::Key,
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, slider, text, toggler},
};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    view_for(app, None)
}

pub(crate) fn deployment_view(app: &App, connections: bool) -> Element<'_, Message> {
    view_for(app, Some(connections))
}

fn view_for(app: &App, deployment: Option<bool>) -> Element<'_, Message> {
    let selected = match deployment {
        Some(true) => SettingsTab::Connections,
        Some(false) => SettingsTab::Deployment,
        None => app.settings_tab,
    };
    let t = |key| app.i18n.text(key);
    let preferences = &app.workspace.preferences;
    let mut tabs = column![].spacing(4);
    let sections = [
        (SettingsTab::General, Key::General, "general"),
        (SettingsTab::Code, Key::Code, "code"),
        (SettingsTab::Panels, Key::Panels, "panels"),
        (SettingsTab::Modules, Key::Modules, "modules"),
        (
            SettingsTab::Connections,
            Key::DeploymentConnections,
            "connections",
        ),
        (
            SettingsTab::Deployment,
            Key::DeploymentProfiles,
            "deployment",
        ),
        (SettingsTab::Shortcuts, Key::KeyboardShortcuts, "shortcuts"),
        (SettingsTab::About, Key::About, "about"),
    ];
    for &(tab, key, id) in &sections {
        if matches!(tab, SettingsTab::Connections | SettingsTab::Deployment) {
            continue;
        }
        tabs = tabs.push(
            container(
                button(text(t(key)).size(13))
                    .padding([8, 12])
                    .width(Length::Fill)
                    .on_press(Message::SelectSettingsTab(tab))
                    .style(theme::action(selected == tab)),
            )
            .id(format!("settings-tab-{id}")),
        );
    }
    let section: Element<'_, Message> = match selected {
        SettingsTab::General => {
            let language = preferences.language.as_deref().unwrap_or("auto");
            let mut languages = row![].spacing(6);
            for (code, label) in [
                ("auto", t(Key::SystemLanguage)),
                ("en", "English".to_owned()),
                ("ru", "Русский".to_owned()),
                ("es", "Español".to_owned()),
            ] {
                languages = languages.push(
                    button(text(label).size(13))
                        .padding([8, 12])
                        .on_press(Message::SetLanguage(code.to_owned()))
                        .style(theme::action(language == code)),
                );
            }
            column![
                setting(
                    t(Key::Language),
                    t(Key::LanguageDescription),
                    languages.wrap().into()
                ),
                setting(
                    t(Key::Appearance),
                    t(Key::AppearanceDescription),
                    row([Appearance::Dark, Appearance::Light].map(|appearance| {
                        button(
                            text(t(match appearance {
                                Appearance::Dark => Key::Dark,
                                Appearance::Light => Key::Light,
                            }))
                            .size(13),
                        )
                        .padding([8, 14])
                        .on_press(Message::SetAppearance(appearance))
                        .style(theme::action(preferences.appearance == appearance))
                        .into()
                    }))
                    .spacing(6)
                    .into()
                ),
            ]
            .spacing(0)
            .into()
        }
        SettingsTab::Code => column![
            setting(
                t(Key::FontSize),
                t(Key::FontSizeDescription),
                row![
                    slider(11..=22, preferences.code_size, Message::SetCodeSize)
                        .width(180)
                        .style(theme::font_slider),
                    text(format!("{} px", preferences.code_size))
                        .size(13)
                        .width(48),
                ]
                .spacing(16)
                .align_y(Alignment::Center)
                .into()
            ),
            setting(
                t(Key::LineNumbers),
                t(Key::LineNumbersDescription),
                toggler(preferences.line_numbers)
                    .style(theme::toggle)
                    .on_toggle(Message::SetLineNumbers)
                    .into()
            ),
        ]
        .spacing(0)
        .into(),
        SettingsTab::Panels => column![
            setting(
                t(Key::Git),
                t(Key::GitDescription),
                toggler(preferences.show_git)
                    .style(theme::toggle)
                    .on_toggle_maybe(app.module_enabled(crate::modules::ModuleId::Git).then_some(
                        |_| Message::TogglePanel(Panel::Module(crate::modules::ModuleId::Git))
                    ))
                    .into()
            ),
            setting(
                t(Key::Pipelines),
                t(Key::PipelinesDescription),
                toggler(preferences.show_pipelines)
                    .style(theme::toggle)
                    .on_toggle_maybe(
                        app.module_enabled(crate::modules::ModuleId::Pipelines)
                            .then_some(|_| Message::TogglePanel(Panel::Module(
                                crate::modules::ModuleId::Pipelines
                            )))
                    )
                    .into()
            ),
        ]
        .spacing(0)
        .into(),
        SettingsTab::Modules => crate::ui::modules::settings(app),
        SettingsTab::Connections => crate::modules::deployment::connections_view(
            &app.modules.deployment,
            &app.workspace.deployment,
            &app.i18n,
        )
        .map(crate::app::modules::deployment_message),
        SettingsTab::Deployment => crate::modules::deployment::profiles_view(
            &app.modules.deployment,
            &app.workspace.deployment,
            &app.workspace.spaces,
            app.workspace.selected_space,
            &app.i18n,
        )
        .map(crate::app::modules::deployment_message),
        SettingsTab::Shortcuts => column![
            shortcut_group(
                t(Key::Code),
                "settings-shortcuts-code",
                vec![
                    setting(
                        t(Key::GoToLine),
                        t(Key::GoToLineDescription),
                        shortcut_control(app, Command::GoToLine),
                    ),
                    setting(
                        t(Key::EditorSave),
                        t(Key::EditorSaveDescription),
                        shortcut_control(app, Command::SaveFile),
                    ),
                    setting(
                        t(Key::CopyCodeSelection),
                        t(Key::CopyCodeSelectionDescription),
                        shortcut_control(app, Command::Copy),
                    ),
                    setting(
                        t(Key::SelectAllCode),
                        t(Key::SelectAllCodeDescription),
                        shortcut_control(app, Command::SelectAll),
                    ),
                ],
            ),
            shortcut_group(
                t(Key::ShortcutTabs),
                "settings-shortcuts-tabs",
                vec![
                    setting(
                        t(Key::CloseTab),
                        t(Key::CloseActiveTabDescription),
                        shortcut_control(app, Command::CloseTab),
                    ),
                    setting(
                        t(Key::ReopenClosedTab),
                        t(Key::ReopenClosedTabDescription),
                        shortcut_control(app, Command::ReopenTab),
                    ),
                ],
            ),
            shortcut_group(
                t(Key::Deployment),
                "settings-shortcuts-deployment",
                vec![
                    setting(
                        t(Key::DeploymentCompare),
                        String::new(),
                        shortcut_control(app, Command::DeploymentCompare)
                    ),
                    setting(
                        t(Key::DeploymentUpload),
                        String::new(),
                        shortcut_control(app, Command::DeploymentUpload)
                    ),
                ]
            ),
        ]
        .spacing(24)
        .into(),
        SettingsTab::About => column![
            setting(
                t(Key::AppTitle),
                t(Key::AboutDescription),
                text(env!("CARGO_PKG_VERSION"))
                    .size(13)
                    .style(theme::MUTED.text())
                    .into(),
            ),
            crate::ui::updates::settings(app)
        ]
        .spacing(24)
        .into(),
    };
    let heading = sections
        .iter()
        .find(|(tab, _, _)| *tab == selected)
        .map_or(Key::General, |(_, key, _)| *key);
    let navigation = container(
        scrollable(tabs.padding(12))
            .style(theme::scroll)
            .height(Length::Fill),
    )
    .id("settings-navigation")
    .width(theme::SETTINGS_SIDEBAR_WIDTH)
    .height(Length::Fill)
    .style(theme::sidebar);
    let body: Element<'_, Message> = if deployment.is_some() {
        container(section)
            .id("settings-content")
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    } else {
        scrollable(
            container(
                column![
                    section,
                    text(t(Key::SettingsSaved))
                        .size(12)
                        .style(theme::MUTED.text())
                ]
                .spacing(20)
                .width(Length::Fill)
                .max_width(theme::SETTINGS_CONTENT_WIDTH),
            )
            .padding(theme::SETTINGS_CONTENT_INSET)
            .width(Length::Fill),
        )
        .id("settings-content")
        .style(theme::scroll)
        .height(Length::Fill)
        .into()
    };
    let content = column![
        container(text(t(heading)).size(20).font(theme::semibold()))
            .id("settings-section-heading")
            .padding([16., theme::SETTINGS_CONTENT_INSET]),
        divider(),
        body,
    ]
    .height(Length::Fill)
    .width(Length::Fill);
    let (base, window) = match deployment {
        Some(true) => (content.into(), app.windows.connections),
        Some(false) => (content.into(), app.windows.deployment),
        None => (
            row![navigation, super::workspace::vertical_divider(), content]
                .height(Length::Fill)
                .width(Length::Fill)
                .into(),
            app.windows.settings,
        ),
    };
    let base = if app.shortcuts.recording.is_none() {
        let dismiss = window.map_or(Message::Navigate(crate::app::Navigation::Projects), |id| {
            Message::WindowShortcut(id, Box::new(Message::DismissDialog))
        });
        crate::ui::context_menu::dismiss_on_escape(base, dismiss)
    } else {
        base
    };
    if deployment.is_some_and(|connections| app.modules.deployment.has_settings_dialog(connections))
        && let Some(dialog) = crate::modules::deployment::settings_dialog(
            &app.modules.deployment,
            &app.workspace.deployment,
            &app.i18n,
        )
    {
        super::modal::overlay(
            base,
            dialog.map(crate::app::modules::deployment_message),
            640.,
            crate::app::modules::deployment_message(crate::modules::deployment::Event::CloseDialog),
        )
    } else {
        base
    }
}

fn shortcut_group<'a>(
    title: String,
    id: &'static str,
    shortcuts: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(14).font(theme::semibold()),
            column(shortcuts).spacing(0),
        ]
        .spacing(8),
    )
    .id(id)
    .width(Length::Fill)
    .into()
}

fn shortcut_control(app: &App, command: Command) -> Element<'_, Message> {
    use crate::app::shortcuts::Event;
    let recording = app.shortcuts.recording == Some(command);
    let label = if recording {
        app.i18n.text(Key::ShortcutRecording)
    } else {
        app.workspace.preferences.shortcuts.binding(command).label()
    };
    let mut controls = row![
        button(text(label).size(13))
            .padding([8, 14])
            .on_press(Message::Shortcut(Event::Record(command)))
            .style(theme::action(recording)),
    ]
    .spacing(8);
    if recording {
        controls = controls.push(
            button(text(app.i18n.text(Key::Cancel)).size(13))
                .padding([8, 14])
                .on_press(Message::Shortcut(Event::Cancel))
                .style(theme::action(false)),
        );
    }
    let mut content = column![controls].spacing(8);
    if recording {
        content = content.push(
            text(app.i18n.text(Key::ShortcutInstructions))
                .size(12)
                .style(theme::MUTED.text()),
        );
        if let Some(error) = app.shortcuts.error {
            content = content.push(text(app.i18n.text(error)).size(13));
        }
    }
    container(content)
        .id(format!("shortcut-{command:?}"))
        .into()
}

pub(crate) fn setting<'a>(
    title: String,
    description: String,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut label = column![text(title).size(14)].spacing(4);
    if !description.is_empty() {
        label = label.push(
            text(description)
                .size(13)
                .line_height(1.3)
                .style(theme::MUTED.text()),
        );
    }
    column![
        row![
            label.width(Length::Fill),
            container(control)
                .width(theme::SETTINGS_CONTROL_WIDTH)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(24)
        .align_y(Alignment::Center)
        .padding([14, 0]),
        divider(),
    ]
    .width(Length::Fill)
    .into()
}
