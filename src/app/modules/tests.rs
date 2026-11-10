use super::*;
use crate::{
    app::SettingsTab,
    i18n::{I18n, Key, Language},
    modules::terminal::{Event as TerminalEvent, Status},
};
use std::{path::PathBuf, rc::Rc};

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.create_space(
        "Work".into(),
        vec![PathBuf::from("first"), PathBuf::from("second")],
    );
    app
}

#[test]
fn deployment_panel_is_independent_and_preserves_source_width_when_hidden() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        let source = app.preview().id;
        for visible in [true, false] {
            let _ = app.update_module(Event::Toggle(ModuleId::Deployment));
            assert_eq!(app.modules.deployment.open, visible);
            assert_eq!(app.preview().id, source);
            assert!(!app.modules.terminal.open);
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1440., 920.),
                app.view(),
            );
            let source = ui
                .find(iced::widget::Id::new("source-preview"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let toggle = ui
                .find(iced::widget::Id::new("deployment-toggle"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(source.x + source.width <= toggle.x);
            let panel = ui
                .find(iced::widget::Id::new("deployment-panel"))
                .ok()
                .and_then(|p| p.visible_bounds());
            assert_eq!(panel.is_some_and(|p| p.width > 0.), visible);
            if visible {
                assert!(panel.unwrap().x >= source.x + source.width);
            }
        }
    }
}

#[test]
fn legacy_disabled_system_module_remains_available_without_losing_tabs() {
    let mut app = app();
    app.workspace
        .preferences
        .disabled_modules
        .insert("terminal".into());
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    assert_eq!(
        app.modules.terminal.active_tab().unwrap().status,
        Status::Starting
    );
    assert_eq!(app.modules.terminal.tabs.len(), 1);
    let request = app.modules.terminal.active.unwrap();
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    assert_eq!(app.modules.terminal.active, Some(request));
    assert_eq!(app.modules.terminal.tabs.len(), 1);
    assert!(app.modules.terminal.open);
}

#[test]
fn sidebar_opens_and_hides_the_terminal_through_module_events() {
    let mut app = app();
    for opened in [true, false] {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: crate::ui::theme::font(),
                ..iced::Settings::default()
            },
            (1080., 720.),
            app.view(),
        );
        ui.click(iced::widget::Id::new("nav-terminal")).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(
            messages.iter().any(|message| matches!(
                message,
                Message::Module(Event::Toggle(ModuleId::Terminal))
            ))
        );
        for message in messages {
            let _ = app.update(message);
        }
        assert_eq!(app.modules.terminal.open, opened);
        assert_eq!(
            app.panes
                .iter()
                .any(|(_, panel)| matches!(panel, crate::app::Panel::Module(ModuleId::Terminal))),
            opened
        );
    }
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    assert!(ui.find(iced::widget::Id::new("nav-terminal")).is_ok());
}

#[test]
fn terminal_controls_hide_without_dropping_tabs_and_close_the_last_tab_with_its_panel() {
    fn click(app: &mut App, id: impl Into<iced::widget::Id>) -> Vec<Message> {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: crate::ui::theme::font(),
                ..iced::Settings::default()
            },
            (1080., 720.),
            app.view(),
        );
        ui.click(id.into()).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        for message in messages.iter().cloned() {
            let _ = app.update(message);
        }
        messages
    }
    let mut app = app();
    assert!(app.modules.terminal.tabs.is_empty());
    click(&mut app, "nav-terminal");
    assert_eq!(app.modules.terminal.tabs.len(), 1);
    let first = app.modules.terminal.active.unwrap();
    app.modules.terminal.tabs[0]
        .parser
        .process(b"retained-output");
    click(&mut app, "terminal-add-tab");
    assert_eq!(app.modules.terminal.tabs.len(), 2);
    let second = app.modules.terminal.active.unwrap();
    click(&mut app, "terminal-hide");
    assert!(!app.modules.terminal.open);
    assert_eq!(app.modules.terminal.tabs.len(), 2);
    click(&mut app, "nav-terminal");
    assert!(app.modules.terminal.open);
    assert_eq!(app.modules.terminal.tabs.len(), 2);
    assert_eq!(
        app.modules.terminal.tabs[0].parser.screen().contents(),
        "retained-output"
    );
    let messages = click(
        &mut app,
        iced::widget::Id::from(format!("terminal-close-tab-{first}")),
    );
    assert!(messages.iter().any(|event| matches!(event, Message::Module(Event::Terminal(TerminalEvent::Close(id))) if *id == first)));
    assert!(!messages.iter().any(|event| matches!(
        event,
        Message::Module(Event::Terminal(TerminalEvent::Select(_)))
    )));
    assert_eq!(app.modules.terminal.active, Some(second));
    assert!(app.modules.terminal.open);
    click(
        &mut app,
        iced::widget::Id::from(format!("terminal-close-tab-{second}")),
    );
    assert!(!app.modules.terminal.open);
    assert!(app.modules.terminal.tabs.is_empty());
    assert!(
        !app.panes
            .iter()
            .any(|(_, panel)| matches!(panel, crate::app::Panel::Module(_)))
    );
    click(&mut app, "nav-terminal");
    assert_eq!(app.modules.terminal.tabs.len(), 1);
    assert!(app.modules.terminal.active.unwrap() > second);
}

#[test]
fn project_switch_invalidates_pending_terminal_and_closes_its_panel() {
    let mut app = app();
    let _ = app.load_directory();
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    let request = app.modules.terminal.active_tab().unwrap().request;
    let _ = app.update(Message::SelectProject(0, 1));
    let _ = app.update_module(Event::Terminal(TerminalEvent::Started(request, Err(()))));
    assert!(app.modules.terminal.tabs.is_empty());
    assert!(!app.modules.terminal.open);
}

#[test]
fn same_project_refresh_keeps_terminal_and_its_output() {
    let mut app = app();
    let _ = app.load_directory();
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    app.modules.terminal.tabs[0].status = Status::Exited;
    app.modules.terminal.tabs[0]
        .parser
        .process(b"retained-output");
    let request = app.modules.terminal.active_tab().unwrap().request;
    let _ = app.load_directory();
    assert!(app.modules.terminal.open);
    assert_eq!(app.modules.terminal.active_tab().unwrap().request, request);
    assert_eq!(
        app.modules
            .terminal
            .active_tab()
            .unwrap()
            .parser
            .screen()
            .contents(),
        "retained-output"
    );
}

#[test]
fn legacy_module_preferences_survive_roundtrip_without_disabling_system_modules() {
    let mut app = app();
    app.workspace
        .preferences
        .disabled_modules
        .extend(["terminal".into(), "future-module".into()]);
    let json = serde_json::to_string(&app.workspace).unwrap();
    let restored: crate::domain::workspace::Workspace = serde_json::from_str(&json).unwrap();
    assert!(restored.preferences.disabled_modules.contains("terminal"));
    assert!(
        restored
            .preferences
            .disabled_modules
            .contains("future-module")
    );
    app.workspace = restored;
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    assert_eq!(app.modules.terminal.tabs.len(), 1);
    let legacy: crate::domain::workspace::Workspace =
        serde_json::from_str(r#"{"spaces":[],"preferences":{"code_size":16}}"#).unwrap();
    assert!(legacy.preferences.disabled_modules.is_empty());
    assert_eq!(legacy.preferences.code_size, 16);
}

#[test]
fn system_module_settings_have_no_disable_controls_in_all_languages_and_themes() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        for appearance in [
            crate::domain::workspace::Appearance::Dark,
            crate::domain::workspace::Appearance::Light,
        ] {
            let mut app = app();
            app.i18n = Rc::new(I18n::new(language));
            app.workspace.preferences.appearance = appearance;
            app.settings_tab = SettingsTab::Modules;
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings {
                    default_font: crate::ui::theme::font(),
                    ..iced::Settings::default()
                },
                (780., 610.),
                crate::ui::screens::settings::view(&app),
            );
            for key in [
                Key::ModulesDescription,
                Key::SystemModule,
                Key::Terminal,
                Key::ModuleEnabled,
            ] {
                assert!(
                    ui.find(app.i18n.text(key))
                        .unwrap()
                        .visible_bounds()
                        .is_some()
                );
            }
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&app.theme())
                    .unwrap()
                    .matches_image(output.join(format!("modules-{language:?}-{appearance:?}")))
                    .unwrap();
            }
            assert!(
                ui.find(iced::widget::Id::new("module-terminal-enabled"))
                    .is_err()
            );
            ui.click(app.i18n.text(Key::Terminal)).unwrap();
            assert!(ui.into_messages().next().is_none());
        }
    }
}

#[test]
fn terminal_panel_fits_the_minimum_workspace_in_all_languages_and_themes() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        for appearance in [
            crate::domain::workspace::Appearance::Dark,
            crate::domain::workspace::Appearance::Light,
        ] {
            let mut app = app();
            app.i18n = Rc::new(I18n::new(language));
            app.workspace.preferences.appearance = appearance;
            let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
            for _ in 0..2 {
                let _ = app.update_module(Event::Terminal(TerminalEvent::Start));
            }
            for tab in &mut app.modules.terminal.tabs {
                tab.status = Status::Exited;
            }
            app.modules.terminal.tabs[2]
                .parser
                .process("\x1b[32mHello · Привет · Hola\x1b[0m\r\nproject> ".as_bytes());
            app.sync_panels();
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings {
                    default_font: crate::ui::theme::font(),
                    ..iced::Settings::default()
                },
                (1080., 720.),
                app.view(),
            );
            let screen = ui
                .find(iced::widget::Id::new("terminal-screen"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(screen.width > 0. && screen.height > 0.);
            assert!(screen.x >= 0. && screen.y >= 0.);
            assert!(screen.x + screen.width <= 1080. && screen.y + screen.height <= 720.);
            for id in ["terminal-hide", "terminal-add-tab", "terminal-close-tab-1"] {
                let bounds = ui
                    .find(iced::widget::Id::new(id))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(bounds.x + bounds.width <= 1080.);
            }
            for request in 1..=3 {
                let tab = ui
                    .find(iced::widget::Id::from(format!("terminal-tab-{request}")))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let close = ui
                    .find(iced::widget::Id::from(format!(
                        "terminal-close-tab-{request}"
                    )))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(close.x >= tab.x && close.y >= tab.y);
                assert!(close.x + close.width <= tab.x + tab.width);
                assert!(close.y + close.height <= tab.y + tab.height);
            }
            let last = ui
                .find(iced::widget::Id::new("terminal-tab-3"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let add = ui
                .find(iced::widget::Id::new("terminal-add-tab"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let gap = add.x - (last.x + last.width);
            assert!(
                (0. ..=10.).contains(&gap),
                "Plus must follow the last terminal tab: {gap}"
            );
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&app.theme())
                    .unwrap()
                    .matches_image(output.join(format!("terminal-{language:?}-{appearance:?}")))
                    .unwrap();
            }
        }
    }
}
