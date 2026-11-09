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
    let mut tabs = row![].spacing(4);
    for (tab, key, id) in [
        (SettingsTab::General, Key::General, "general"),
        (SettingsTab::Code, Key::Code, "code"),
        (SettingsTab::Panels, Key::Panels, "panels"),
        (SettingsTab::Shortcuts, Key::KeyboardShortcuts, "shortcuts"),
        (SettingsTab::About, Key::About, "about"),
    ] {
        tabs = tabs.push(
            container(
                button(text(t(key)).size(13))
                    .padding([8, 12])
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
        ]
        .spacing(16)
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
    column![
        container(
            column![
                text(t(Key::Settings)).size(28).font(theme::semibold()),
                text(t(Key::SettingsDescription))
                    .size(15)
                    .style(theme::MUTED.text()),
                tabs,
                divider(),
            ]
            .spacing(16)
            .max_width(760)
        )
        .padding([24, 40])
        .center_x(Length::Fill),
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
            .padding([0, 40])
            .center_x(Length::Fill)
        )
        .id("settings-content")
        .style(theme::scroll)
        .height(Length::Fill),
    ]
    .height(Length::Fill)
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
    row![
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
    .spacing(24)
    .align_y(Alignment::Center)
    .padding([12, 0])
    .into()
}
