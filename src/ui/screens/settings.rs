use super::workspace::divider;
use crate::{
    app::{App, Message, Panel, SettingsTab},
    domain::workspace::Appearance,
    i18n::Key,
    ui::theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, slider, text, toggler},
};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let t = |key| app.i18n.text(key);
    let preferences = &app.workspace.preferences;
    let mut tabs = column![].spacing(4);
    let sections = [
        (SettingsTab::General, Key::General, "general"),
        (SettingsTab::Code, Key::Code, "code"),
        (SettingsTab::Panels, Key::Panels, "panels"),
        (SettingsTab::Shortcuts, Key::KeyboardShortcuts, "shortcuts"),
        (SettingsTab::About, Key::About, "about"),
    ];
    for &(tab, key, id) in &sections {
        tabs = tabs.push(
            container(
                button(text(t(key)).size(13))
                    .padding([8, 12])
                    .width(Length::Fill)
                    .on_press(Message::SelectSettingsTab(tab))
                    .style(theme::action(app.settings_tab == tab)),
            )
            .id(format!("settings-tab-{id}")),
        );
    }
    let section: Element<'_, Message> = match app.settings_tab {
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
                    languages.into()
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
            .spacing(16)
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
        .spacing(16)
        .into(),
        SettingsTab::Panels => column![
            setting(
                t(Key::Git),
                t(Key::GitDescription),
                toggler(preferences.show_git)
                    .style(theme::toggle)
                    .on_toggle(|_| Message::TogglePanel(Panel::Git))
                    .into()
            ),
            setting(
                t(Key::Pipelines),
                t(Key::PipelinesDescription),
                toggler(preferences.show_pipelines)
                    .style(theme::toggle)
                    .on_toggle(|_| Message::TogglePanel(Panel::Pipelines))
                    .into()
            ),
        ]
        .spacing(16)
        .into(),
        SettingsTab::Shortcuts => column![
            shortcut_group(
                t(Key::Code),
                "settings-shortcuts-code",
                vec![
                    setting(
                        t(Key::GoToLine),
                        t(Key::GoToLineDescription),
                        shortcut_badge("Command + L", "Ctrl + G"),
                    ),
                    setting(
                        t(Key::CopyCodeSelection),
                        t(Key::CopyCodeSelectionDescription),
                        shortcut_badge("Command + C", "Ctrl + C"),
                    ),
                    setting(
                        t(Key::SelectAllCode),
                        t(Key::SelectAllCodeDescription),
                        shortcut_badge("Command + A", "Ctrl + A"),
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
                        shortcut_badge("Command + W", "Ctrl + W"),
                    ),
                    setting(
                        t(Key::ReopenClosedTab),
                        t(Key::ReopenClosedTabDescription),
                        shortcut_badge("Command + Shift + T", "Ctrl + Shift + T"),
                    ),
                ],
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
        .find(|(tab, _, _)| *tab == app.settings_tab)
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
    let content = column![
        container(
            column![
                container(text(t(heading)).size(24).font(theme::semibold()))
                    .id("settings-section-heading"),
                text(t(Key::SettingsDescription))
                    .size(14)
                    .style(theme::MUTED.text()),
                divider(),
            ]
            .spacing(12)
        )
        .padding([24, 24]),
        scrollable(
            container(
                column![
                    section,
                    text(t(Key::SettingsSaved))
                        .size(12)
                        .style(theme::FAINT.text())
                ]
                .spacing(24)
                .max_width(760)
            )
            .padding([0, 24])
            .center_x(Length::Fill)
        )
        .id("settings-content")
        .style(theme::scroll)
        .height(Length::Fill),
    ]
    .height(Length::Fill)
    .width(Length::Fill);
    row![navigation, super::workspace::vertical_divider(), content]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn shortcut_group<'a>(
    title: String,
    id: &'static str,
    shortcuts: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(17).font(theme::semibold()),
            column(shortcuts).spacing(8),
        ]
        .spacing(8),
    )
    .id(id)
    .width(Length::Fill)
    .into()
}

fn shortcut_badge(mac: &'static str, other: &'static str) -> Element<'static, Message> {
    container(
        text(if cfg!(target_os = "macos") {
            mac
        } else {
            other
        })
        .size(13),
    )
    .padding([8, 14])
    .style(theme::badge)
    .into()
}

fn setting<'a>(
    title: String,
    description: String,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        column![
            text(title).size(15),
            text(description)
                .size(13)
                .line_height(1.4)
                .style(theme::MUTED.text())
        ]
        .spacing(6)
        .width(Length::Fill),
        control
    ]
    .spacing(12)
    .width(Length::Fill)
    .padding([12, 0])
    .into()
}
