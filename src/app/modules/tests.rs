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
fn module_panels_start_collapsed_and_git_header_preserves_module_state_when_collapsed() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        app.sync_panels();
        for module in crate::modules::MODULES {
            assert!(app.module_enabled(module.id));
            assert!(!app.module_open(module.id));
        }
        assert_eq!(app.panes.len(), 1);
        let _ = app.update_module(Event::Toggle(ModuleId::Git));
        app.modules.git.query = "retained search".into();
        let source = app.preview().id;
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        ui.click(iced::widget::Id::new("git-hide")).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Module(Event::Git(crate::modules::git::Event::Hide))
        )));
        for message in messages {
            let _ = app.update(message);
        }
        assert!(!app.module_open(ModuleId::Git));
        assert!(app.module_enabled(ModuleId::Git));
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.preview().id, source);
        let _ = app.update_module(Event::Toggle(ModuleId::Git));
        assert!(app.module_open(ModuleId::Git));
        assert_eq!(app.modules.git.query, "retained search");
    }
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
fn disabled_terminal_rejects_events_and_reenable_starts_a_new_session() {
    let mut app = app();
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    let request = app.modules.terminal.active.unwrap();
    let _ = app.update_module(Event::SetEnabled(ModuleId::Terminal, false));
    assert!(app.modules.terminal.tabs.is_empty());
    assert_eq!(
        iced::advanced::subscription::into_recipes(app.module_subscription()).len(),
        0
    );
    let _ = app.update_module(Event::Terminal(TerminalEvent::Start));
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    assert!(app.modules.terminal.tabs.is_empty());
    let _ = app.update_module(Event::SetEnabled(ModuleId::Terminal, true));
    let _ = app.update_module(Event::Toggle(ModuleId::Terminal));
    let fresh = app.modules.terminal.active.unwrap();
    assert!(fresh > request);
    let _ = app.update_module(Event::Terminal(TerminalEvent::Started(request, Err(()))));
    assert_eq!(
        app.modules.terminal.active_tab().unwrap().status,
        Status::Starting
    );
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
            .any(|(_, panel)| matches!(panel, crate::app::Panel::Module(ModuleId::Terminal)))
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
fn module_preferences_survive_roundtrip_and_old_settings_enable_every_module() {
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
    assert!(app.modules.terminal.tabs.is_empty());
    let legacy: crate::domain::workspace::Workspace =
        serde_json::from_str(r#"{"spaces":[],"preferences":{"code_size":16}}"#).unwrap();
    assert!(legacy.preferences.disabled_modules.is_empty());
    assert_eq!(legacy.preferences.code_size, 16);
    app.workspace = legacy;
    for module in crate::modules::MODULES {
        assert!(app.module_enabled(module.id));
    }
}

#[test]
fn module_settings_offer_activation_controls_in_all_languages_and_themes() {
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
            for key in [Key::ModulesDescription, Key::SystemModule, Key::Terminal] {
                assert!(
                    ui.find(app.i18n.text(key))
                        .unwrap()
                        .visible_bounds()
                        .is_some()
                );
            }
            let terminal = ui
                .find(iced::widget::Id::new("module-terminal"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            let deployment = ui
                .find(iced::widget::Id::new("module-deployment"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(terminal.height <= 60.);
            assert!(deployment.height <= 60.);
            assert!((deployment.x - terminal.x).abs() < 1.);
            assert!((deployment.y - terminal.y - terminal.height).abs() < 1.);
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&app.theme())
                    .unwrap()
                    .matches_image(output.join(format!("modules-{language:?}-{appearance:?}")))
                    .unwrap();
            }
            assert!(
                ui.find(iced::widget::Id::new("module-terminal-enabled"))
                    .is_ok()
            );
            ui.click(iced::widget::Id::new("module-terminal-enabled"))
                .unwrap();
            assert!(ui.into_messages().any(|message| matches!(
                message,
                Message::Module(Event::SetEnabled(ModuleId::Terminal, false))
            )));
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

#[test]
fn every_module_can_be_disabled_without_changing_saved_configuration_or_open_files() {
    let mut app = app();
    let source = app.preview().id;
    let deployment = serde_json::to_value(&app.workspace.deployment).unwrap();
    let spaces = serde_json::to_value(&app.workspace.spaces).unwrap();
    app.workspace.preferences.show_git = true;
    app.workspace.preferences.show_pipelines = true;
    app.workspace
        .preferences
        .disabled_modules
        .insert("future-module".into());
    for module in crate::modules::MODULES {
        assert!(app.module_enabled(module.id));
        let _ = app.update_module(Event::SetEnabled(module.id, false));
        let _ = app.update_module(Event::Toggle(module.id));
        assert!(!app.module_open(module.id));
    }
    assert_eq!(app.preview().id, source);
    assert_eq!(
        serde_json::to_value(&app.workspace.deployment).unwrap(),
        deployment
    );
    assert_eq!(serde_json::to_value(&app.workspace.spaces).unwrap(), spaces);
    assert!(app.workspace.preferences.show_git);
    assert!(app.workspace.preferences.show_pipelines);
    assert_eq!(app.panes.len(), 1);
    for subscription in [
        app.module_subscription(),
        app.pipeline_subscription(),
        app.jobs_subscription(),
        app.change_subscription(),
        app.project_change_subscription(),
    ] {
        assert!(iced::advanced::subscription::into_recipes(subscription).is_empty());
    }
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1280., 800.), app.view());
    for id in [
        "nav-git",
        "nav-pipelines",
        "nav-terminal",
        "deployment-toggle",
        "file-panel-changes",
    ] {
        assert!(ui.find(iced::widget::Id::from(id)).is_err(), "{id}");
    }
    drop(ui);
    let restored = serde_json::from_str(&serde_json::to_string(&app.workspace).unwrap()).unwrap();
    app.workspace = restored;
    for module in crate::modules::MODULES {
        assert!(!app.module_enabled(module.id));
        let _ = app.update_module(Event::SetEnabled(module.id, true));
        assert!(app.module_enabled(module.id));
    }
    assert!(
        app.workspace
            .preferences
            .disabled_modules
            .contains("future-module")
    );
}

#[test]
fn disabled_modules_reject_commands_and_late_results_after_reenable() {
    use crate::{domain::Failure, modules::pipelines::PipelineState};
    let mut app = app();
    let generation = app.generation;
    let pipeline_request = app.modules.pipelines.pipeline_request;
    let _ = app.update_module(Event::SetEnabled(ModuleId::Pipelines, false));
    let _ = app.update(Message::RefreshPipelines);
    assert!(!app.modules.pipelines.pipeline_refreshing);
    let _ = app.update_module(Event::SetEnabled(ModuleId::Pipelines, true));
    let _ = app.update(Message::PipelinesLoaded(
        generation,
        pipeline_request,
        crate::modules::pipelines::transport::Loaded {
            runs: Err(Failure::Access),
            username: Some("obsolete".into()),
        },
    ));
    assert!(matches!(
        app.modules.pipelines.pipelines,
        PipelineState::Idle
    ));
    assert!(app.account_username.is_none());
    let _ = app.update_module(Event::SetEnabled(ModuleId::Git, false));
    let _ = app.update(git_message(crate::modules::git::Event::Search(
        "blocked".into(),
    )));
    let _ = app.update(Message::Git(crate::modules::git::actions::Message::Fetch));
    assert!(app.modules.git.query.is_empty());
    assert!(!app.modules.git.git_actions.pending);
    let _ = app.update_module(Event::SetEnabled(ModuleId::Deployment, false));
    let _ = app.update(deployment_message(
        crate::modules::deployment::Event::Toggle,
    ));
    let _ = app.update(deployment_message(
        crate::modules::deployment::Event::CurrentFile(false),
    ));
    assert!(!app.modules.deployment.open);
    assert!(!app.modules.deployment.busy);
}

#[test]
fn restored_disabled_modules_can_be_enabled_from_the_settings_window() {
    let mut saved = app().workspace;
    saved.preferences.disabled_modules = crate::modules::MODULES
        .iter()
        .map(|module| module.id.key().to_owned())
        .collect();
    let restored = serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let _ = app.update(Message::Loaded(Ok((restored, None))));
    let _ = app.update(Message::Navigate(crate::app::Navigation::Settings));
    let window = app.windows.settings.unwrap();
    let click = |app: &mut App, id: String| {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (1120., 760.),
            app.view_window(window),
        );
        ui.click(iced::widget::Id::from(id)).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(!messages.is_empty());
        for message in messages {
            let _ = app.update(message);
        }
    };
    click(&mut app, "settings-tab-modules".into());
    assert_eq!(app.settings_tab, SettingsTab::Modules);
    for module in crate::modules::MODULES {
        assert!(!app.module_enabled(module.id));
        click(&mut app, format!("module-{}-enabled", module.id.key()));
        assert!(app.module_enabled(module.id));
    }
    assert!(app.workspace.preferences.disabled_modules.is_empty());
}

#[test]
fn module_panel_controls_emit_typed_events() {
    let mut app = app();
    app.workspace.preferences.show_git = true;
    app.sync_panels();
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1280., 800.), app.view());
    ui.click(iced::widget::Id::new("git-refresh")).unwrap();
    assert!(ui.into_messages().any(|message| matches!(
        message,
        Message::Module(Event::Git(crate::modules::git::Event::Refresh))
    )));
}
