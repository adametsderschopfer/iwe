use super::*;
use crate::i18n::{I18n, Language};
use iced::keyboard::{Key as KeyboardKey, Modifiers, key};
use std::rc::Rc;

#[test]
fn deployment_shortcuts_use_physical_keys_for_russian_layouts() {
    for (letter, code, compare) in [("С", key::Code::KeyC, true), ("Ф", key::Code::KeyA, false)] {
        let message = key_message(
            &KeyboardKey::Character(letter.into()),
            key::Physical::Code(code),
            Modifiers::COMMAND | Modifiers::SHIFT,
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

fn escape(id: window::Id, status: Status) -> Event {
    Event::Key(
        id,
        keyboard::Event::KeyPressed {
            key: KeyboardKey::Named(key::Named::Escape),
            modified_key: KeyboardKey::Named(key::Named::Escape),
            physical_key: key::Physical::Code(key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::NONE,
            repeat: false,
            text: None,
        },
        status,
    )
}

#[tokio::test]
async fn escape_closes_only_the_focused_secondary_window() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task, window::Action as WindowAction};
    for deployment in [false, true] {
        let mut app = App::new(Rc::new(I18n::new(Language::English)));
        let main = window::Id::unique();
        let settings = window::Id::unique();
        let server = window::Id::unique();
        app.window_id = Some(main);
        app.windows.settings = Some(settings);
        app.windows.deployment = Some(server);
        let focused = if deployment { server } else { settings };
        let tabs = app.tabs.files.len();
        let action = app.update_shortcuts(escape(focused, Status::Ignored));
        let actions = task::into_stream(action).unwrap().collect::<Vec<_>>().await;
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::Window(WindowAction::Close(id)) if id == focused));
        assert_eq!(app.tabs.files.len(), tabs);
        assert_eq!(app.window_id, Some(main));
        let _ = app.update(Message::Window(super::super::windows::Event::Closed(
            focused,
        )));
        assert_eq!(
            app.windows.settings,
            if deployment { Some(settings) } else { None }
        );
        assert_eq!(
            app.windows.deployment,
            if deployment { None } else { Some(server) }
        );
    }
}

#[test]
fn escape_dismisses_nested_dialogs_and_shortcut_recording_before_closing_windows() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let settings = window::Id::unique();
    let connections = window::Id::unique();
    app.windows.settings = Some(settings);
    app.windows.connections = Some(connections);
    app.settings_tab = SettingsTab::Shortcuts;
    let _ = app.update_shortcuts(Event::Record(Command::CloseTab));
    assert_eq!(
        app.update_shortcuts(escape(settings, Status::Ignored))
            .units(),
        0
    );
    assert!(app.shortcuts.recording.is_none());
    assert_eq!(app.windows.settings, Some(settings));
    let _ = app.update(super::super::modules::deployment_message(
        crate::modules::deployment::Event::NewKey,
    ));
    assert_eq!(
        app.update_shortcuts(escape(connections, Status::Ignored))
            .units(),
        0
    );
    assert!(!app.modules.deployment.has_dialog());
    assert_eq!(app.windows.connections, Some(connections));
    assert_eq!(
        app.update_shortcuts(escape(settings, Status::Captured))
            .units(),
        0
    );
    assert_eq!(app.windows.settings, Some(settings));
}

fn workspace_app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.spaces = vec![crate::domain::workspace::Space {
        name: "Work".into(),
        projects: vec![crate::domain::workspace::Project {
            name: "Alpha".into(),
            directory: std::path::PathBuf::from("/tmp/alpha"),
            pinned: false,
        }],
        deployments: Vec::new(),
    }];
    app.workspace.select_project(0, 0);
    app
}

fn shift(id: window::Id) -> Event {
    Event::Key(
        id,
        keyboard::Event::KeyPressed {
            key: KeyboardKey::Named(key::Named::Shift),
            modified_key: KeyboardKey::Named(key::Named::Shift),
            physical_key: key::Physical::Code(key::Code::ShiftLeft),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::SHIFT,
            repeat: false,
            text: None,
        },
        Status::Ignored,
    )
}

#[test]
fn pressing_shift_twice_within_the_window_opens_search_everywhere() {
    let mut app = workspace_app();
    let id = window::Id::unique();
    app.window_id = Some(id);
    let _ = app.update_shortcuts(shift(id));
    let _ = app.update_shortcuts(shift(id));
    assert!(app.search_everywhere.dialog.is_some());
    assert_eq!(
        app.search_everywhere
            .dialog
            .as_ref()
            .map(|dialog| dialog.scope),
        Some(crate::app::search_everywhere::Scope::All)
    );
}

#[test]
fn spaced_or_interrupted_shift_presses_do_not_open_search_everywhere() {
    let mut app = workspace_app();
    let id = window::Id::unique();
    app.window_id = Some(id);

    let _ = app.update_shortcuts(shift(id));
    std::thread::sleep(std::time::Duration::from_millis(350));
    let _ = app.update_shortcuts(shift(id));
    assert!(app.search_everywhere.dialog.is_none());

    app.shortcuts.last_shift = None;
    let _ = app.update_shortcuts(shift(id));
    let _ = app.update_shortcuts(press(id, "a", key::Code::KeyA, Modifiers::NONE));
    let _ = app.update_shortcuts(shift(id));
    assert!(app.search_everywhere.dialog.is_none());

    app.shortcuts.last_shift = None;
    let settings = window::Id::unique();
    app.windows.settings = Some(settings);
    let _ = app.update_shortcuts(shift(settings));
    let _ = app.update_shortcuts(shift(settings));
    assert!(app.search_everywhere.dialog.is_none());
}
