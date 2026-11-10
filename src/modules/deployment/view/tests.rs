use super::*;
use crate::{i18n::Language, modules::deployment::transport::Entry};
use iced::{Event as IcedEvent, mouse};

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
