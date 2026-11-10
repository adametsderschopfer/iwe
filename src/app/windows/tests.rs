use super::*;
use crate::{
    app::Navigation,
    i18n::{I18n, Language},
};
use std::rc::Rc;

#[test]
fn settings_reuses_one_window_and_preserves_workspace_navigation() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.window_id = Some(window::Id::unique());
    app.navigation = Navigation::Files;
    assert!(app.update(Message::Navigate(Navigation::Settings)).units() > 0);
    let id = app.windows.settings.unwrap();
    assert_ne!(Some(id), app.window_id);
    assert_eq!(app.navigation, Navigation::Files);
    assert!(app.update(Message::Navigate(Navigation::Settings)).units() > 0);
    assert_eq!(app.windows.settings, Some(id));
    let _ = app.update(Message::Window(Event::Closed(id)));
    assert!(app.windows.settings.is_none());
    assert!(app.window_id.is_some());
    let _ = app.update(Message::Navigate(Navigation::Settings));
    assert_ne!(app.windows.settings, Some(id));
}

#[test]
fn settings_shortcuts_never_close_source_tabs_or_open_workspace_dialogs() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.window_id = Some(window::Id::unique());
    let _ = app.open_settings();
    let id = app.windows.settings.unwrap();
    let tabs = app.tabs.files.len();
    let _ = app.update(Message::WindowShortcut(
        id,
        Box::new(Message::CloseActiveTab),
    ));
    assert_eq!(app.tabs.files.len(), tabs);
    let _ = app.update(Message::WindowShortcut(
        id,
        Box::new(Message::GoToLine(
            super::super::line_navigation::Event::Open,
        )),
    ));
    assert!(app.line_navigation.is_none());
    assert!(app.window_id.is_some());
}

#[test]
fn settings_window_fits_all_languages_without_the_project_sidebar() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        let _ = app.open_settings();
        let id = app.windows.settings.unwrap();
        assert_eq!(app.window_title(id), app.i18n.text(Key::Settings));
        for size in [
            settings_settings().size,
            settings_settings().min_size.unwrap(),
        ] {
            for tab in [
                super::super::SettingsTab::General,
                super::super::SettingsTab::Code,
                super::super::SettingsTab::Panels,
                super::super::SettingsTab::Shortcuts,
                super::super::SettingsTab::About,
            ] {
                app.settings_tab = tab;
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings {
                        default_font: theme::font(),
                        ..Default::default()
                    },
                    size,
                    app.view_window(id),
                );
                assert!(ui.find(iced::widget::Id::new("nav-settings")).is_err());
                let title = ui
                    .find(app.i18n.text(Key::SettingsDescription))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(title.x >= 0. && title.y >= 0.);
                assert!(title.x + title.width <= size.width);
                assert!(title.y + title.height <= size.height);
                let navigation = ui
                    .find(iced::widget::Id::new("settings-navigation"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let heading = ui
                    .find(iced::widget::Id::new("settings-section-heading"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(heading.x >= navigation.x + navigation.width);
                let mut previous = None;
                for name in ["general", "code", "panels", "shortcuts", "about"] {
                    let bounds = ui
                        .find(iced::widget::Id::from(format!("settings-tab-{name}")))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    assert!(
                        bounds.x >= navigation.x
                            && bounds.x + bounds.width <= navigation.x + navigation.width
                    );
                    if let Some(previous) = previous {
                        assert!(bounds.y > previous);
                    }
                    previous = Some(bounds.y);
                }
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                {
                    let header = ui
                        .find(iced::widget::Id::new("settings-window-header"))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    assert_eq!(header.y, 0.);
                    assert_eq!(header.height, theme::SETTINGS_HEADER_HEIGHT);
                    assert!(navigation.y >= header.y + header.height);
                }
                ui.click(iced::widget::Id::new("settings-tab-about"))
                    .unwrap();
                assert!(ui.into_messages().any(|message| matches!(
                    message,
                    Message::SelectSettingsTab(super::super::SettingsTab::About)
                )));
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn settings_content_covers_the_native_titlebar_without_hiding_system_controls() {
    let settings = settings_settings();
    assert!(settings.decorations);
    assert!(settings.transparent);
    assert!(settings.platform_specific.fullsize_content_view);
    assert!(settings.platform_specific.titlebar_transparent);
    assert!(settings.platform_specific.title_hidden);
}
