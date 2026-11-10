use super::*;
use crate::i18n::{I18n, Language};
use iced::keyboard::{Key as KeyboardKey, Modifiers, key};
use std::rc::Rc;

#[test]
fn deployment_shortcuts_use_physical_keys_for_russian_layouts() {
    for (letter, code, compare) in [("с", key::Code::KeyC, true), ("м", key::Code::KeyV, false)] {
        let message = key_message(
            &KeyboardKey::Character(letter.into()),
            key::Physical::Code(code),
            Modifiers::COMMAND | Modifiers::ALT,
            &crate::domain::shortcuts::Shortcuts::default(),
        );
        assert!(
            matches!(message, Some(Message::Module(crate::app::modules::Event::Deployment(crate::modules::deployment::Event::CurrentFile(value)))) if value == compare)
        );
    }
}

fn press(id: window::Id, name: &str, code: key::Code, modifiers: Modifiers) -> Event {
    Event::Key(
        id,
        keyboard::Event::KeyPressed {
            key: KeyboardKey::Character(name.into()),
            modified_key: KeyboardKey::Character(name.into()),
            physical_key: key::Physical::Code(code),
            location: keyboard::Location::Standard,
            modifiers,
            repeat: false,
            text: None,
        },
        Status::Ignored,
    )
}

#[test]
fn recording_rejects_conflicts_then_applies_new_binding_without_executing_it() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let id = window::Id::unique();
    app.windows.settings = Some(id);
    app.settings_tab = SettingsTab::Shortcuts;
    let original = app
        .workspace
        .preferences
        .shortcuts
        .binding(Command::CloseTab);
    let _ = app.update_shortcuts(Event::Record(Command::CloseTab));
    let _ = app.update_shortcuts(press(id, "c", key::Code::KeyC, Modifiers::COMMAND));
    assert_eq!(app.shortcuts.error, Some(Key::ShortcutConflict));
    assert_eq!(app.shortcuts.recording, Some(Command::CloseTab));
    assert_eq!(
        app.workspace
            .preferences
            .shortcuts
            .binding(Command::CloseTab),
        original
    );
    let _ = app.update_shortcuts(press(id, "k", key::Code::KeyK, Modifiers::COMMAND));
    assert!(app.shortcuts.recording.is_none());
    assert_eq!(
        app.workspace
            .preferences
            .shortcuts
            .binding(Command::CloseTab)
            .key,
        "K"
    );
    assert!(app.windows.settings.is_some());
}

#[test]
fn recording_ignores_other_windows_and_cancels_without_changes() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let id = window::Id::unique();
    app.windows.settings = Some(id);
    app.settings_tab = SettingsTab::Shortcuts;
    let _ = app.update_shortcuts(Event::Record(Command::GoToLine));
    let _ = app.update_shortcuts(press(
        window::Id::unique(),
        "k",
        key::Code::KeyK,
        Modifiers::COMMAND,
    ));
    assert_eq!(app.shortcuts.recording, Some(Command::GoToLine));
    let _ = app.update_shortcuts(press(id, "k", key::Code::KeyK, Modifiers::NONE));
    assert_eq!(app.shortcuts.error, Some(Key::ShortcutInvalid));
    let _ = app.update_shortcuts(Event::Cancel);
    assert!(app.shortcuts.recording.is_none());
    assert_eq!(
        app.workspace
            .preferences
            .shortcuts
            .binding(Command::GoToLine),
        Command::GoToLine.default_binding()
    );
}

#[test]
fn reassignment_changes_runtime_dispatch_and_releases_previous_combination() {
    let mut bindings = crate::domain::shortcuts::Shortcuts::default();
    let mut custom = Command::CloseTab.default_binding();
    custom.key = "K".into();
    bindings.assign(Command::CloseTab, custom).unwrap();
    assert!(
        key_message(
            &KeyboardKey::Character("w".into()),
            key::Physical::Code(key::Code::KeyW),
            Modifiers::COMMAND,
            &bindings
        )
        .is_none()
    );
    assert!(matches!(
        key_message(
            &KeyboardKey::Character("л".into()),
            key::Physical::Code(key::Code::KeyK),
            Modifiers::COMMAND,
            &bindings
        ),
        Some(Message::CloseActiveTab)
    ));
}

#[test]
fn shortcut_buttons_and_conflicts_render_in_all_languages() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.navigation = Navigation::Settings;
        app.settings_tab = SettingsTab::Shortcuts;
        let label = app
            .workspace
            .preferences
            .shortcuts
            .binding(Command::GoToLine)
            .label();
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 900.), app.view());
        ui.click(label).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(
            messages.iter().any(|message| matches!(
                message,
                Message::Shortcut(Event::Record(Command::GoToLine))
            ))
        );
        for message in messages {
            let _ = app.update(message);
        }
        app.shortcuts.error = Some(Key::ShortcutConflict);
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 900.), app.view());
        for key in [
            Key::ShortcutRecording,
            Key::ShortcutInstructions,
            Key::ShortcutConflict,
            Key::Cancel,
        ] {
            assert!(
                ui.find(app.i18n.text(key))
                    .unwrap()
                    .visible_bounds()
                    .is_some()
            );
        }
    }
}
