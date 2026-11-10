use super::settings::{connection_form, keys_view};
use super::*;
use crate::{i18n::Language, modules::deployment::transport::Entry};
use iced::{Event as IcedEvent, mouse};

#[test]
fn profile_tabs_fit_translations_share_a_baseline_and_select_each_page() {
    use crate::modules::deployment::state::ProfileTab;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        for active in [
            ProfileTab::Connection,
            ProfileTab::Mappings,
            ProfileTab::Exclusions,
        ] {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings {
                    default_font: theme::font(),
                    ..Default::default()
                },
                (560., 80.),
                settings::profile_tabs(active, &i18n),
            );
            let mut previous_right = 0.;
            let mut baseline = None;
            for name in ["connection", "mappings", "exclusions"] {
                let tab = ui
                    .find(iced::widget::Id::from(format!(
                        "deployment-profile-tab-{name}"
                    )))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let label = ui
                    .find(iced::widget::Id::from(format!(
                        "deployment-profile-tab-label-{name}"
                    )))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let indicator = ui
                    .find(iced::widget::Id::from(format!(
                        "deployment-profile-tab-indicator-{name}"
                    )))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!((tab.x - previous_right).abs() < 1.);
                assert!((tab.width - label.width - 24.).abs() < 1.);
                assert!((label.x - tab.x - 12.).abs() < 1.);
                assert!((indicator.x - tab.x).abs() < 1.);
                assert!((indicator.width - tab.width).abs() < 1.);
                assert_eq!(indicator.y + indicator.height, tab.y + tab.height);
                assert!(tab.x + tab.width <= 560.);
                if let Some(baseline) = baseline {
                    assert_eq!(indicator.y, baseline);
                }
                baseline = Some(indicator.y);
                previous_right = tab.x + tab.width;
                ui.click(iced::widget::Id::from(format!(
                    "deployment-profile-tab-{name}"
                )))
                .unwrap();
            }
            let selected: Vec<_> = ui
                .into_messages()
                .filter_map(|event| match event {
                    Event::ProfileTab(tab) => Some(tab),
                    _ => None,
                })
                .collect();
            assert_eq!(
                selected,
                [
                    ProfileTab::Connection,
                    ProfileTab::Mappings,
                    ProfileTab::Exclusions
                ]
            );
        }
        let mut narrow = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (220., 80.),
            settings::profile_tabs(ProfileTab::Connection, &i18n),
        );
        let viewport = narrow
            .find(iced::widget::Id::new("deployment-profile-tabs"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(viewport.width, 220.);
        narrow.point_at(viewport.center());
        narrow.simulate([IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: -600., y: 0. },
        })]);
        narrow
            .click(iced::widget::Id::new("deployment-profile-tab-exclusions"))
            .unwrap();
        assert!(
            narrow
                .into_messages()
                .any(|event| matches!(event, Event::ProfileTab(ProfileTab::Exclusions)))
        );
    }
}

#[test]
fn remote_files_open_on_double_click_and_show_context_actions() {
    let mut state = State::default();
    let entry = Entry {
        path: "/hello.txt".into(),
        name: "hello.txt".into(),
        directory: false,
        symlink: false,
        size: 4,
    };
    state.tree.insert("/".into(), vec![entry]);
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (300., 500.),
        tree(&state, "/", 0),
    );
    ui.click("hello.txt").unwrap();
    ui.click("hello.txt").unwrap();
    assert!(ui.into_messages().any(
        |message| matches!(message, Event::Run(Operation::Preview(path)) if path == "/hello.txt")
    ));
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (300., 500.),
        tree(&state, "/", 0),
    );
    let bounds = ui.find("hello.txt").unwrap().visible_bounds().unwrap();
    ui.point_at(bounds.center());
    ui.simulate([IcedEvent::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Right,
    ))]);
    assert!(
        ui.into_messages()
            .any(|message| matches!(message, Event::Menu(entry, _) if entry.path == "/hello.txt"))
    );
}

#[test]
fn connection_forms_render_all_languages_without_raw_errors() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        let mut settings = Connections::default();
        let _ = state.update(Event::NewConnection, &mut settings, &mut vec![], None);
        state.error = Some(Error::Credentials);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (600., 1500.),
            connection_form(&state, &settings, &i18n),
        );
        for key in [
            Key::DeploymentHost,
            Key::DeploymentUsername,
            Key::DeploymentCredentialsError,
        ] {
            assert!(ui.find(i18n.text(key)).unwrap().visible_bounds().is_some());
        }
    }
}

#[test]
fn connection_test_feedback_precedes_the_button_and_keeps_recovery_controls() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        let mut settings = Connections::default();
        let _ = state.update(Event::NewConnection, &mut settings, &mut vec![], None);
        state.operation_label = Some(Key::DeploymentTest);
        state.test_connection = state.connection.clone();
        for (name, busy, error, status, message) in [
            ("loading", true, None, None, Key::DeploymentChecking),
            (
                "success",
                false,
                None,
                Some(Key::DeploymentConnected),
                Key::DeploymentConnected,
            ),
            (
                "failure",
                false,
                Some(Error::Connection),
                None,
                Key::DeploymentConnectionError,
            ),
            (
                "trust",
                false,
                Some(Error::HostUnknown("SHA256:test-fingerprint".into())),
                None,
                Key::DeploymentUnknownHost,
            ),
        ] {
            state.busy = busy;
            state.test_result = if busy {
                None
            } else {
                Some(error.clone().map_or(Ok(()), Err))
            };
            state.error = error;
            state.status = status;
            for appearance in [iced::Theme::Dark, iced::Theme::Light] {
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings {
                        default_font: theme::font(),
                        ..Default::default()
                    },
                    (600., 700.),
                    connection_form(&state, &settings, &i18n),
                );
                let feedback = ui
                    .find(iced::widget::Id::new("deployment-test-status"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let button = ui
                    .find(iced::widget::Id::new("deployment-test-button"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(
                    ui.find(i18n.text(message))
                        .unwrap()
                        .visible_bounds()
                        .is_some()
                );
                assert!(feedback.y + feedback.height <= button.y);
                assert_eq!(feedback.x, button.x);
                assert!(ui.find(i18n.text(Key::DeploymentBytes)).is_err());
                if name == "trust" {
                    ui.click(i18n.text(Key::DeploymentTrust)).unwrap();
                }
                if busy {
                    ui.click(i18n.text(Key::DeploymentCancel)).unwrap();
                }
                ui.click(iced::widget::Id::new("deployment-test-button"))
                    .unwrap();
                if let Some(output) = std::env::var_os("IWE_TEST_CONNECTION_RENDER_DIR") {
                    let output = std::path::PathBuf::from(output);
                    std::fs::create_dir_all(&output).unwrap();
                    ui.snapshot(&appearance)
                        .unwrap()
                        .matches_image(
                            output.join(format!("connection-{language:?}-{appearance:?}-{name}")),
                        )
                        .unwrap();
                }
                let messages: Vec<_> = ui.into_messages().collect();
                assert_eq!(
                    messages.iter().any(|event| matches!(event, Event::Test)),
                    !busy
                );
                assert_eq!(
                    messages.iter().any(|event| matches!(event, Event::Cancel)),
                    busy
                );
                assert_eq!(
                    messages
                        .iter()
                        .any(|event| matches!(event, Event::TrustHost)),
                    name == "trust"
                );
            }
        }
    }
}

#[test]
fn key_creation_is_a_separate_localized_dialog() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        let mut settings = Connections::default();
        let mut list = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (600., 600.),
            keys_view(&state, &settings, &i18n),
        );
        assert!(list.find(i18n.text(Key::DeploymentKeyPath)).is_err());
        list.click(i18n.text(Key::DeploymentAddIdentity)).unwrap();
        assert!(
            list.into_messages()
                .any(|message| matches!(message, Event::NewKey))
        );
        let _ = state.update(Event::NewKey, &mut settings, &mut vec![], None);
        let mut dialog = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (600., 600.),
            settings_dialog(&state, &settings, &i18n).unwrap(),
        );
        assert!(
            dialog
                .find(i18n.text(Key::DeploymentKeyPath))
                .unwrap()
                .visible_bounds()
                .is_some()
        );
        dialog.click(i18n.text(Key::DeploymentCancel)).unwrap();
        assert!(
            dialog
                .into_messages()
                .any(|message| matches!(message, Event::CloseDialog))
        );
    }
}

#[test]
fn activity_controls_work_without_the_remote_tree_in_all_languages() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        state.busy = true;
        state.activity_visible = true;
        state.operation_name = "Production".into();
        state.operation_label = Some(Key::DeploymentUpload);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (480., 40.),
            activity(&state, &i18n),
        );
        ui.click(i18n.text(Key::DeploymentUpload)).unwrap();
        assert!(
            ui.into_messages()
                .any(|event| matches!(event, Event::ToggleActivity))
        );
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (380., 360.),
            activity_details(&state, &i18n),
        );
        ui.click(i18n.text(Key::DeploymentCancel)).unwrap();
        assert!(
            ui.into_messages()
                .any(|event| matches!(event, Event::Cancel))
        );
        let mut state = state;
        state.busy = false;
        state.error = Some(Error::Overwrite);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (380., 360.),
            activity_details(&state, &i18n),
        );
        ui.click(i18n.text(Key::DeploymentReplace)).unwrap();
        assert!(
            ui.into_messages()
                .any(|event| matches!(event, Event::ConfirmDownload))
        );
    }
}

#[test]
fn server_folder_dialog_shows_directories_and_selects_without_opening_files() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        state.browsing = true;
        let directory = Entry {
            path: "/site".into(),
            name: "site".into(),
            directory: true,
            symlink: false,
            size: 0,
        };
        state.selected = Some(directory.clone());
        state.tree.insert(
            "/".into(),
            vec![
                directory,
                Entry {
                    path: "/file.txt".into(),
                    name: "file.txt".into(),
                    directory: false,
                    symlink: false,
                    size: 5,
                },
                Entry {
                    path: "/link".into(),
                    name: "link".into(),
                    directory: true,
                    symlink: true,
                    size: 0,
                },
            ],
        );
        let connections = Connections::default();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (700., 540.),
            settings::settings_dialog(&state, &connections, &i18n).unwrap(),
        );
        assert!(ui.find("site").unwrap().visible_bounds().is_some());
        assert!(ui.find("file.txt").is_err());
        assert!(ui.find("link").is_err());
        ui.click(i18n.text(Key::DeploymentUseFolder)).unwrap();
        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Event::UseRoot(path) if path == "/site"))
        );
    }
}

#[test]
fn remote_menu_limits_mapped_actions_and_keeps_the_selected_remote_path() {
    use crate::domain::deployment::Mapping;
    for mapped in [false, true] {
        let i18n = I18n::new(Language::English);
        let project = std::env::temp_dir().join("iwe-menu-test");
        let profiles = vec![Profile {
            id: "profile".into(),
            name: "Server".into(),
            project: project.clone(),
            connection: "connection".into(),
            root: "/srv".into(),
            mappings: vec![Mapping {
                local: "".into(),
                remote: "mapped".into(),
            }],
            ..Default::default()
        }];
        let path = if mapped {
            "/srv/mapped/file.txt"
        } else {
            "/srv/outside.txt"
        };
        let mut state = State::default();
        state.menu = Some(Entry {
            path: path.into(),
            name: "file.txt".into(),
            directory: false,
            symlink: false,
            size: 10,
        });
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (800., 600.),
            overlay(
                &state,
                &i18n,
                iced::widget::space()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
                &profiles,
                Some(&project),
                |event| event,
            ),
        );
        for key in [
            Key::DeploymentUpload,
            Key::DeploymentDownload,
            Key::DeploymentCompareLocal,
            Key::DeploymentOpenLocal,
            Key::DeploymentOpenRemote,
            Key::CopyAbsolutePath,
        ] {
            ui.click(i18n.text(key)).unwrap();
        }
        let events: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, Event::RemoteCompare(p) if p == path)),
            mapped
        );
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, Event::RemoteUpload(p) if p == path)),
            mapped
        );
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, Event::OpenLocal(p) if p == path)),
            mapped
        );
        assert_eq!(
            events.iter().any(
                |event| matches!(event, Event::Run(Operation::Download(p, false)) if p == path)
            ),
            mapped
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Run(Operation::Preview(p)) if p == path))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::CopyRemotePath(p) if p == path))
        );
    }
}

#[test]
fn target_picker_selects_profiles_and_cancels_in_all_languages() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let i18n = I18n::new(language);
        let mut state = State::default();
        state.target = Some((
            Operation::Upload("file.txt".into()),
            vec![
                ("dev".into(), "Development".into()),
                ("prod".into(), "Production".into()),
            ],
        ));
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            (520., 500.),
            target_dialog(&state, &i18n).unwrap(),
        );
        ui.click("Production").unwrap();
        ui.click(i18n.text(Key::DeploymentCancel)).unwrap();
        let events: Vec<_> = ui.into_messages().collect();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::ChooseTarget(id) if id == "prod"))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::CloseDialog))
        );
    }
}

#[test]
fn connection_test_status_does_not_leak_to_another_connection_or_transfer() {
    let i18n = I18n::new(Language::English);
    let settings = Connections::default();
    let mut state = State::default();
    let connection = crate::domain::deployment::Connection {
        id: "first".into(),
        ..Default::default()
    };
    state.connection = Some(connection.clone());
    state.test_connection = Some(connection);
    state.test_result = Some(Ok(()));
    state.operation_label = Some(Key::DeploymentTest);
    state.error = Some(Error::Mapping);
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (600., 1000.),
        connection_form(&state, &settings, &i18n),
    );
    assert!(ui.find(i18n.text(Key::DeploymentConnected)).is_ok());
    drop(ui);
    state.connection.as_mut().unwrap().id = "other".into();
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (600., 1000.),
        connection_form(&state, &settings, &i18n),
    );
    assert!(
        ui.find(iced::widget::Id::new("deployment-test-status"))
            .is_err()
    );
    drop(ui);
    state.connection = state.test_connection.clone();
    state.operation_label = Some(Key::DeploymentUpload);
    state.busy = true;
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        (600., 1000.),
        connection_form(&state, &settings, &i18n),
    );
    assert!(
        ui.find(iced::widget::Id::new("deployment-test-status"))
            .is_err()
    );
}
