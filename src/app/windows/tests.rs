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
                super::super::SettingsTab::Modules,
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
                    .find(iced::widget::Id::new("settings-section-heading"))
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
                let first_label = match tab {
                    super::super::SettingsTab::General => Key::Language,
                    super::super::SettingsTab::Code => Key::FontSize,
                    super::super::SettingsTab::Panels => Key::Git,
                    super::super::SettingsTab::Modules => Key::ModulesDescription,
                    super::super::SettingsTab::Shortcuts => Key::GoToLine,
                    super::super::SettingsTab::About => Key::AppTitle,
                    _ => unreachable!(),
                };
                let label = ui
                    .find(app.i18n.text(first_label))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!((label.x - heading.x - theme::SETTINGS_CONTENT_INSET).abs() < 1.);
                assert!(
                    label.y < heading.y + heading.height + 80.,
                    "{tab:?}: {label:?}, {heading:?}"
                );
                if let Some(output) =
                    std::env::var_os("IWE_SETTINGS_RENDER_DIR").map(std::path::PathBuf::from)
                {
                    std::fs::create_dir_all(&output).unwrap();
                    ui.snapshot(&app.theme())
                        .unwrap()
                        .matches_image(
                            output.join(format!("settings-{language:?}-{tab:?}-{}", size.width)),
                        )
                        .unwrap();
                }
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

#[cfg(target_os = "windows")]
#[test]
fn secondary_windows_draw_a_visible_frame() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let _ = app.open_settings();
    let id = app.windows.settings.unwrap();
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        settings_settings().size,
        app.view_window(id),
    );
    assert!(
        ui.find(iced::widget::Id::new("settings-window-frame"))
            .is_ok()
    );
    let frame = theme::window_frame(&iced::Theme::Dark);
    assert_eq!(frame.border.width, 1.);
    assert_ne!(frame.border.color, iced::Color::TRANSPARENT);
    assert!(frame.background.is_some());
}

#[test]
fn deployment_editors_fit_the_settings_window_and_keep_keys_inside_connections() {
    use crate::{
        app::SettingsTab,
        domain::deployment::{Authentication, Connection, Mapping, Profile},
    };
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        let project = std::env::temp_dir().join("sample-shop");
        app.workspace
            .create_space("Example".into(), vec![project.clone()]);
        let directory = app.workspace.directory().cloned();
        app.modules
            .deployment
            .sync_context(app.workspace.selected_space, directory.as_deref());
        let connection = Connection {
            id: "staging".into(),
            name: "Staging".into(),
            host: "staging.example.invalid".into(),
            port: 22,
            username: "deploy".into(),
            authentication: Authentication::Agent,
            ..Default::default()
        };
        app.workspace
            .deployment
            .connections
            .push(connection.clone());
        app.modules.deployment.connection = Some(connection);
        app.modules.deployment.port = "22".into();
        let profile = Profile {
            id: "development".into(),
            name: "Development".into(),
            project,
            connection: "staging".into(),
            root: "/srv/shop".into(),
            mappings: vec![Mapping {
                local: "public".into(),
                remote: "www".into(),
            }],
            excluded: vec!["cache".into()],
            default: true,
        };
        app.workspace.spaces[0].deployments.push(profile.clone());
        app.modules.deployment.profile = Some(profile);
        app.modules.deployment.operation_label = Some(Key::DeploymentCompare);
        app.modules.deployment.operation_path = "/sample-shop/index.html".into();
        app.modules.deployment.status = Some(Key::DeploymentComparisonReady);
        let _ = app.open_deployment_settings(false);
        let _ = app.open_deployment_settings(true);
        for tab in [SettingsTab::Connections, SettingsTab::Deployment] {
            let id = if tab == SettingsTab::Connections {
                app.windows.connections.unwrap()
            } else {
                app.windows.deployment.unwrap()
            };
            for size in [
                settings_settings().size,
                settings_settings().min_size.unwrap(),
            ] {
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings {
                        default_font: theme::font(),
                        ..Default::default()
                    },
                    size,
                    app.view_window(id),
                );
                assert!(
                    ui.find(iced::widget::Id::new("settings-tab-deployment-keys"))
                        .is_err()
                );
                assert!(
                    ui.find(iced::widget::Id::new("settings-navigation"))
                        .is_err()
                );
                assert!(ui.find(app.i18n.text(Key::SettingsSaved)).is_err());
                let save = ui
                    .find(app.i18n.text(Key::DeploymentSave))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let name = ui
                    .find(app.i18n.text(Key::DeploymentName))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(name.x >= 176.);
                assert!(save.x + save.width <= size.width && save.y + save.height <= size.height);
                if tab == SettingsTab::Connections {
                    assert!(ui.find(app.i18n.text(Key::DeploymentKeys)).is_err());
                    let warning = ui
                        .find(app.i18n.text(Key::DeploymentConnectionsHelp))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    assert!(warning.y > name.y && warning.y + warning.height <= save.y);
                }
            }
            if tab == SettingsTab::Deployment {
                for key in [Key::DeploymentMappings, Key::DeploymentExclusions] {
                    let mut ui = iced_test::Simulator::with_size(
                        iced::Settings::default(),
                        settings_settings().size,
                        app.view_window(id),
                    );
                    ui.click(app.i18n.text(key)).unwrap();
                    for message in ui.into_messages() {
                        let _ = app.update(message);
                    }
                    let mut ui = iced_test::Simulator::with_size(
                        iced::Settings::default(),
                        settings_settings().size,
                        app.view_window(id),
                    );
                    assert!(ui.find("/sample-shop/index.html").is_err());
                    assert!(
                        ui.find(app.i18n.text(Key::DeploymentComparisonReady))
                            .is_err()
                    );
                }
                app.modules.deployment.profile_tab = Default::default();
            }
            if let Some(output) =
                std::env::var_os("IWE_DEPLOYMENT_RENDER_DIR").map(std::path::PathBuf::from)
            {
                std::fs::create_dir_all(&output).unwrap();
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings {
                        default_font: theme::font(),
                        ..Default::default()
                    },
                    settings_settings().size,
                    app.view_window(id),
                );
                ui.snapshot(&iced::Theme::Dark)
                    .unwrap()
                    .matches_image(output.join(format!("deployment-{language:?}-{tab:?}")))
                    .unwrap();
                drop(ui);
                if tab == SettingsTab::Deployment {
                    for key in [Key::DeploymentMappings, Key::DeploymentExclusions] {
                        let mut ui = iced_test::Simulator::with_size(
                            iced::Settings {
                                default_font: theme::font(),
                                ..Default::default()
                            },
                            settings_settings().min_size.unwrap(),
                            app.view_window(id),
                        );
                        ui.click(app.i18n.text(key)).unwrap();
                        for message in ui.into_messages() {
                            let _ = app.update(message);
                        }
                        let mut capture = iced_test::Simulator::with_size(
                            iced::Settings {
                                default_font: theme::font(),
                                ..Default::default()
                            },
                            settings_settings().min_size.unwrap(),
                            app.view_window(id),
                        );
                        capture
                            .snapshot(&iced::Theme::Dark)
                            .unwrap()
                            .matches_image(output.join(format!("deployment-{language:?}-{key:?}")))
                            .unwrap();
                    }
                } else {
                    let _ = app.update_module(crate::app::modules::Event::Deployment(
                        crate::modules::deployment::Event::OpenKeys,
                    ));
                    let _ = app.update_module(crate::app::modules::Event::Deployment(
                        crate::modules::deployment::Event::NewKey,
                    ));
                    let mut capture = iced_test::Simulator::with_size(
                        iced::Settings {
                            default_font: theme::font(),
                            ..Default::default()
                        },
                        settings_settings().min_size.unwrap(),
                        app.view_window(id),
                    );
                    capture
                        .snapshot(&iced::Theme::Dark)
                        .unwrap()
                        .matches_image(output.join(format!("deployment-{language:?}-KeyDialog")))
                        .unwrap();
                    drop(capture);
                    let _ = app.update_module(crate::app::modules::Event::Deployment(
                        crate::modules::deployment::Event::CloseDialog,
                    ));
                    let _ = app.update_module(crate::app::modules::Event::Deployment(
                        crate::modules::deployment::Event::CloseDialog,
                    ));
                }
            }
        }
    }
}

#[test]
fn deployment_window_is_separate_reused_and_closes_independently() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let _ = app.open_settings();
    let settings = app.windows.settings.unwrap();
    let _ = app.open_deployment_settings(false);
    let deployment = app.windows.deployment.unwrap();
    let _ = app.open_deployment_settings(true);
    let connections = app.windows.connections.unwrap();
    assert_ne!(settings, deployment);
    assert_ne!(connections, deployment);
    assert_ne!(connections, settings);
    assert_eq!(
        app.window_title(deployment),
        app.i18n.text(Key::DeploymentProfiles)
    );
    let _ = app.open_deployment_settings(false);
    assert_eq!(app.windows.deployment, Some(deployment));
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (1120., 760.),
        app.view_window(settings),
    );
    assert!(
        ui.find(iced::widget::Id::new("settings-tab-deployment"))
            .is_err()
    );
    assert!(
        ui.find(iced::widget::Id::new("settings-tab-connections"))
            .is_err()
    );
    drop(ui);
    let _ = app.update(Message::Window(Event::Closed(settings)));
    assert_eq!(app.windows.deployment, Some(deployment));
    assert!(app.windows.settings.is_none());
    let _ = app.update(Message::Window(Event::Closed(deployment)));
    assert!(app.windows.deployment.is_none());
    assert_eq!(app.windows.connections, Some(connections));
}

#[test]
fn server_editor_lists_other_spaces_and_saves_project_moves() {
    use crate::{
        app::modules::deployment_message,
        domain::deployment::{Connection, Mapping, Profile},
        modules::deployment::Event as DeploymentEvent,
    };
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let first = std::env::temp_dir().join("first-project");
    let second = std::env::temp_dir().join("second-project");
    app.workspace
        .create_space("First".into(), vec![first.clone()]);
    app.workspace
        .create_space("Second".into(), vec![second.clone()]);
    app.workspace.selected_space = Some(0);
    app.workspace.selected_project = Some(0);
    app.workspace.deployment.connections.push(Connection {
        id: "connection".into(),
        ..Connection::default()
    });
    app.workspace.spaces[1].deployments.push(Profile {
        id: "server".into(),
        name: "Shared server".into(),
        project: second,
        connection: "connection".into(),
        root: "/site".into(),
        mappings: vec![Mapping {
            local: "public".into(),
            remote: "www".into(),
        }],
        default: true,
        ..Profile::default()
    });
    let _ = app.update(Message::SelectSettingsTab(
        crate::app::SettingsTab::Deployment,
    ));
    let id = app.windows.deployment.unwrap();
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        settings_settings().size,
        app.view_window(id),
    );
    assert!(ui.find("Shared server").is_ok());
    assert!(ui.find("second-project").is_ok());
    drop(ui);
    let _ = app.update(deployment_message(DeploymentEvent::ProfileProject(
        0,
        first.clone(),
    )));
    assert_eq!(app.workspace.spaces[1].deployments.len(), 1);
    let _ = app.update(deployment_message(DeploymentEvent::SaveProfile));
    assert!(app.workspace.spaces[1].deployments.is_empty());
    let saved = &app.workspace.spaces[0].deployments[0];
    assert_eq!(saved.id, "server");
    assert_eq!(saved.project, first);
    assert_eq!(saved.mappings[0].local, std::path::Path::new("public"));
    assert!(!saved.default);
}

#[test]
fn escape_in_secondary_window_closes_only_the_top_dialog() {
    use iced::keyboard::{self, key::Named};
    for nested in [false, true] {
        let mut app = App::new(Rc::new(I18n::new(Language::English)));
        let _ = app.open_deployment_settings(true);
        let id = app.windows.connections.unwrap();
        app.modules.deployment.key_dialog = nested;
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (1120., 760.),
            app.view_window(id),
        );
        ui.simulate([iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            modified_key: keyboard::Key::Named(Named::Escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::NONE,
            repeat: false,
            text: None,
        })]);
        let events: Vec<_> = ui.into_messages().collect();
        assert_eq!(events.len(), 1);
        if nested {
            assert!(matches!(
                events[0],
                Message::Module(crate::app::modules::Event::Deployment(
                    crate::modules::deployment::Event::CloseDialog
                ))
            ));
            for event in events {
                let _ = app.update(event);
            }
            assert!(!app.modules.deployment.key_dialog);
            // The captured keyboard subscription must not close the parent after dismissal.
            let _ = app.update_shortcuts(crate::app::shortcuts::Event::Key(
                id,
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Escape),
                    modified_key: keyboard::Key::Named(Named::Escape),
                    physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::NONE,
                    repeat: false,
                    text: None,
                },
                iced::event::Status::Captured,
            ));
            assert_eq!(app.windows.connections, Some(id));
        } else {
            assert!(
                matches!(&events[0], Message::WindowShortcut(window, message) if *window == id && matches!(**message, Message::DismissDialog))
            );
        }
    }
}
