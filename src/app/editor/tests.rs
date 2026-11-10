use super::*;
use crate::{
    i18n::{I18n, Language},
    ui::code::Document,
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.create_space(
        "Work".into(),
        vec![PathBuf::from("project"), PathBuf::from("other")],
    );
    app.loaded = true;
    let _ = app.open_file(PathBuf::from("project/file.txt"));
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::new("original\r\n".into(), "txt")));
    app
}

fn edit(app: &mut App, value: &str) {
    let id = app.preview().id;
    let _ = app.update_editor(Event::Action(id, text_editor::Action::SelectAll));
    let _ = app.update_editor(Event::Action(
        id,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(value.into()))),
    ));
}

#[test]
fn local_edit_does_not_change_disk_snapshot_and_refresh_cannot_replace_draft() {
    let mut app = app();
    edit(&mut app, "draft\r\n");
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "original\r\n"
    );
    assert!(app.editor_buffer().unwrap().document.dirty());
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::plain("external".into())));
    assert_eq!(app.editor_buffer().unwrap().content.text(), "draft\r\n");
    let _ = app.update_editor(Event::Undo(id, false));
    assert_eq!(app.editor_buffer().unwrap().content.text(), "original\r\n");
}

#[test]
fn close_and_project_switch_require_a_decision() {
    let mut app = app();
    edit(&mut app, "draft");
    let id = app.preview().id;
    let _ = app.update(Message::CloseTab(id));
    assert!(app.editor.pending.is_some());
    assert_eq!(app.preview().id, id);
    let _ = app.update_editor(Event::Cancel);
    let _ = app.update(Message::SelectProject(0, 1));
    assert_eq!(app.workspace.selected_project, Some(0));
    assert!(app.editor.pending.is_some());
    let _ = app.update_editor(Event::Cancel);
    let _ = app.update(Message::CloseTab(id));
    let _ = app.update_editor(Event::Discard);
    assert!(app.editor.pending.is_none());
    assert_ne!(app.preview().id, id);
}

#[test]
fn save_completion_keeps_changes_typed_during_save_and_failures_keep_drafts() {
    let mut app = app();
    edit(&mut app, "submitted");
    let id = app.preview().id;
    let _ = app.save_editor(id);
    let token = app.editor_buffer().unwrap().token;
    edit(&mut app, "newer");
    let _ = app.update_editor(Event::Saved(token, Ok(())));
    assert_eq!(app.editor_buffer().unwrap().document.saved, "submitted");
    assert_eq!(app.editor_buffer().unwrap().content.text(), "newer");
    assert!(app.editor_buffer().unwrap().document.dirty());
    let _ = app.save_editor(id);
    let token = app.editor_buffer().unwrap().token;
    let _ = app.update_editor(Event::Saved(token, Err(Key::EditorConflict)));
    assert_eq!(app.editor_buffer().unwrap().content.text(), "newer");
    assert_eq!(
        app.editor_buffer().unwrap().error,
        Some(Key::EditorConflict)
    );
}

#[test]
fn reopening_remote_file_preserves_its_draft_and_original_profile() {
    let mut app = app();
    let _ = app.open_remote_preview(
        "/srv/file.txt".into(),
        "/srv/file.txt".into(),
        ("stage".into(), "Stage".into()),
        Arc::new("server".into()),
        None,
    );
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::plain("server".into())));
    edit(&mut app, "remote draft");
    let _ = app.open_remote_preview(
        "/srv/file.txt".into(),
        "/srv/file.txt".into(),
        ("stage".into(), "Stage".into()),
        Arc::new("external".into()),
        None,
    );
    assert_eq!(app.preview().id, id);
    assert_eq!(app.editor_buffer().unwrap().content.text(), "remote draft");
    assert!(
        matches!(&app.editor_buffer().unwrap().target, Target::Remote { profile, .. } if profile == "stage")
    );
}

#[test]
fn editor_keyboard_input_and_save_work_with_russian_physical_keys() {
    use iced::{
        Event as IcedEvent,
        keyboard::{self, Key as KeyboardKey, Modifiers, key},
        widget::Id,
    };
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        let id = app.preview().id;
        let messages = {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1080., 720.),
                app.view(),
            );
            ui.click(Id::from(format!("text-editor-{id}"))).unwrap();
            ui.typewrite("x");
            ui.simulate([IcedEvent::Keyboard(keyboard::Event::KeyPressed {
                key: KeyboardKey::Character("ы".into()),
                modified_key: KeyboardKey::Character("ы".into()),
                physical_key: key::Physical::Code(key::Code::KeyS),
                location: keyboard::Location::Standard,
                modifiers: Modifiers::COMMAND,
                repeat: false,
                text: None,
            })]);
            ui.into_messages().collect::<Vec<_>>()
        };
        assert!(messages.iter().any(|message| matches!(message, Message::Editor(Event::Action(_, action)) if action.is_edit())));
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, Message::Editor(Event::Save)))
        );
        for message in messages {
            let _ = app.update(message);
        }
        assert!(app.editor_buffer().unwrap().document.dirty());
        let token = app.editor_buffer().unwrap().token;
        let _ = app.update_editor(Event::Saved(token, Err(Key::EditorSaveFailed)));
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR") {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1080., 720.),
                app.view(),
            );
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(PathBuf::from(output).join(format!("editor-buffer-{language:?}")))
                .unwrap();
        }
        let _ = app.update(Message::CloseActiveTab);
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        assert!(
            ui.find(app.i18n.text(Key::EditorUnsavedTitle))
                .unwrap()
                .visible_bounds()
                .is_some()
        );
        ui.click(app.i18n.text(Key::Cancel)).unwrap();
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR") {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(PathBuf::from(output).join(format!("editor-{language:?}")))
                .unwrap();
        }
        assert!(
            ui.into_messages()
                .any(|message| matches!(message, Message::Editor(Event::Cancel)))
        );
    }
}

#[test]
fn native_close_preserves_dirty_buffers_and_settings_do_not_save_them() {
    let mut app = app();
    edit(&mut app, "draft");
    let window = iced::window::Id::unique();
    app.window_id = Some(window);
    let _ = app.update_window(super::super::windows::Event::Close(window));
    assert!(app.editor.pending.is_some());
    let _ = app.update_editor(Event::Cancel);
    app.navigation = super::super::Navigation::Settings;
    let _ = app.update(Message::Editor(Event::Save));
    assert!(!app.editor_buffer().unwrap().saving);
    assert_eq!(app.editor_buffer().unwrap().content.text(), "draft");
}

#[tokio::test]
async fn save_and_close_writes_the_selected_file_without_saving_other_drafts() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let root = tempfile::tempdir().unwrap();
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace
        .create_space("Test".into(), vec![root.path().into()]);
    app.loaded = true;
    let mut ids = Vec::new();
    for name in ["one.txt", "two.txt"] {
        let path = root.path().join(name);
        std::fs::write(&path, "original").unwrap();
        let _ = app.open_file(path);
        let id = app.preview().id;
        let _ = app.finish_preview(id, Ok(Document::plain("original".into())));
        edit(&mut app, name);
        ids.push(id);
    }
    let _ = app.update(Message::CloseTab(ids[0]));
    let mut actions = task::into_stream(app.update_editor(Event::SaveAll)).unwrap();
    while let Some(action) = actions.next().await {
        if let Action::Output(message) = action {
            let _ = app.update(message);
        }
    }
    assert_eq!(
        std::fs::read_to_string(root.path().join("one.txt")).unwrap(),
        "one.txt"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("two.txt")).unwrap(),
        "original"
    );
    assert!(!app.tabs.files.iter().any(|tab| tab.id == ids[0]));
    assert!(app.editor.pending.is_none());
    assert_eq!(app.editor_buffer().unwrap().content.text(), "two.txt");
    assert!(app.editor_buffer().unwrap().document.dirty());
}
