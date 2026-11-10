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
fn failed_saves_require_a_decision_before_close_and_project_switch() {
    let mut app = app();
    edit(&mut app, "draft");
    let id = app.preview().id;
    let _ = app.save_editor(id);
    let token = app.editor_buffer().unwrap().token;
    let _ = app.update_editor(Event::Saved(token, Err(Key::EditorConflict)));
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
    assert!(app.editor_buffer().unwrap().saving);
    let token = app.editor_buffer().unwrap().token;
    let _ = app.update_editor(Event::Saved(token, Err(Key::EditorSaveFailed)));
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
    let mut actions = task::into_stream(app.update(Message::CloseTab(ids[0]))).unwrap();
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

#[test]
fn autosave_ignores_old_timers_and_resumes_newer_edits_after_completion() {
    let mut app = app();
    let id = app.preview().id;
    edit(&mut app, "first");
    let old = app.editor_buffer().unwrap().autosave;
    edit(&mut app, "second");
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, old));
    assert!(!app.editor_buffer().unwrap().saving);
    let _ = app.update_editor(Event::Autosave(id, revision));
    let token = app.editor_buffer().unwrap().token;
    assert_eq!(app.editor_buffer().unwrap().snapshot, "second");
    edit(&mut app, "third");
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, revision));
    assert_eq!(app.editor_buffer().unwrap().token, token);
    let _ = app.update_editor(Event::Saved(token, Ok(())));
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, revision));
    assert_eq!(app.editor_buffer().unwrap().snapshot, "third");
    assert_eq!(app.editor_buffer().unwrap().document.saved, "second");
}

#[test]
fn autosave_errors_stop_retries_even_after_more_typing() {
    let mut app = app();
    let id = app.preview().id;
    edit(&mut app, "draft");
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, revision));
    let token = app.editor_buffer().unwrap().token;
    let _ = app.update_editor(Event::Saved(token, Err(Key::EditorConflict)));
    edit(&mut app, "retained");
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, revision));
    assert!(!app.editor_buffer().unwrap().saving);
    assert_eq!(app.editor_buffer().unwrap().content.text(), "retained");
    assert_eq!(
        app.notifications.items.back().unwrap().message,
        Key::EditorConflict
    );
    let _ = app.update_editor(Event::Save);
    assert!(app.editor_buffer().unwrap().saving);
    assert!(app.editor_buffer().unwrap().error.is_none());
}

#[tokio::test]
async fn autosave_writes_inactive_file_after_delay_and_undo_cancels_changes() {
    use iced::futures::StreamExt;
    use iced_test::runtime::{Action, task};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("file.txt");
    std::fs::write(&path, "original").unwrap();
    let mut app = app();
    let _ = app.open_file(path.clone());
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::plain("original".into())));
    edit(&mut app, "saved automatically");
    let timer = app.schedule_autosave(id);
    let _ = app.open_file(root.path().join("other.txt"));
    app.navigation = super::super::Navigation::Settings;
    let mut timer = task::into_stream(timer).unwrap();
    let Action::Output(message) = timer.next().await.unwrap() else {
        panic!("Expected autosave");
    };
    let mut save = task::into_stream(app.update(message)).unwrap();
    let Action::Output(message) = save.next().await.unwrap() else {
        panic!("Expected save result");
    };
    let _ = app.update(message);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "saved automatically"
    );
    assert!(
        !app.tabs
            .files
            .iter()
            .find(|tab| tab.id == id)
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .protected()
    );
    app.navigation = super::super::Navigation::Files;
    let _ = app.select_tab(id);
    edit(&mut app, "discard by undo");
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Undo(id, false));
    let _ = app.update_editor(Event::Autosave(id, revision));
    assert!(!app.editor_buffer().unwrap().protected());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "saved automatically"
    );
}

#[test]
fn remote_autosave_waits_for_transfer_without_reporting_a_save_failure() {
    let mut app = app();
    let _ = app.open_remote_preview(
        "/test.txt".into(),
        "/test.txt".into(),
        ("stage".into(), "Stage".into()),
        Arc::new("server".into()),
        None,
    );
    let id = app.preview().id;
    let _ = app.finish_preview(id, Ok(Document::plain("server".into())));
    edit(&mut app, "draft");
    app.modules.deployment.busy = true;
    let revision = app.editor_buffer().unwrap().autosave;
    let _ = app.update_editor(Event::Autosave(id, revision));
    assert!(!app.editor_buffer().unwrap().saving);
    assert!(app.editor_buffer().unwrap().error.is_none());
    assert_eq!(app.editor_buffer().unwrap().content.text(), "draft");
    app.modules.deployment.busy = false;
    let _ = app.update_editor(Event::Autosave(id, revision));
    // The absent test profile fails validation, proving that the deferred save was attempted.
    assert!(app.editor_buffer().unwrap().error.is_some());
}

#[test]
fn editor_line_numbers_follow_scroll_and_settings_without_a_save_toolbar() {
    use iced::{Theme, widget::Id};
    let mut app = app();
    let id = app.preview().id;
    let text = (1..=200)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    let _ = app.finish_preview(id, Ok(Document::plain(text)));
    app.workspace.preferences.line_numbers = true;
    for scrolled in [false, true] {
        if scrolled {
            let _ = app.update_editor(Event::Action(id, text_editor::Action::Scroll { lines: 40 }));
        }
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        assert!(
            ui.find(Id::new("editor-line-numbers"))
                .unwrap()
                .visible_bounds()
                .is_some()
        );
        assert!(ui.find(app.i18n.text(Key::EditorSave)).is_err());
        let snapshot = ui.snapshot(&Theme::Dark).unwrap();
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR") {
            snapshot
                .matches_image(PathBuf::from(output).join(format!("autosave-lines-{scrolled}")))
                .unwrap();
        }
        drop(ui);
        app.editor_buffer().unwrap().content.with_editor(|editor| {
            let lines = editor.buffer().layout_runs().collect::<Vec<_>>();
            assert!(!lines.is_empty());
            assert_eq!(lines[0].line_i, if scrolled { 40 } else { 0 });
            assert!(lines.len() < 100);
            for pair in lines.windows(2) {
                assert_eq!(pair[1].line_i, pair[0].line_i + 1);
                assert!((pair[1].line_top - pair[0].line_top - pair[0].line_height).abs() < 0.1);
            }
        });
    }
    app.workspace.preferences.line_numbers = false;
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    assert!(ui.find(Id::new("editor-line-numbers")).is_err());
}

#[test]
fn deployment_flushes_local_drafts_before_starting_and_keeps_failed_drafts() {
    use crate::{
        app::modules::deployment_message,
        domain::deployment::{Connection, Mapping, Profile},
        modules::deployment::{Event as DeploymentEvent, Operation},
    };
    for failed in [false, true] {
        for compare in [false, true] {
            let mut app = app();
            let project = std::env::temp_dir().join("iwe-deployment-flush");
            app.workspace.spaces[0].projects[0].directory = project.clone();
            app.tabs.files.clear();
            let path = project.join("file.txt");
            let _ = app.open_file(path.clone());
            let id = app.preview().id;
            let _ = app.finish_preview(id, Ok(Document::plain("original".into())));
            app.workspace.deployment.connections.push(Connection {
                id: "connection".into(),
                ..Default::default()
            });
            app.workspace.spaces[0].deployments.push(Profile {
                id: "profile".into(),
                name: "Server".into(),
                project: project.clone(),
                connection: "connection".into(),
                root: "/srv".into(),
                mappings: vec![Mapping::default()],
                default: true,
                ..Default::default()
            });
            edit(&mut app, "latest draft");
            let event = DeploymentEvent::Run(if compare {
                Operation::Compare(path.clone())
            } else {
                Operation::Upload(path.clone())
            });
            let _ = app.update(deployment_message(event));
            assert!(app.editor.pending.is_some());
            assert!(!app.modules.deployment.busy);
            assert!(app.editor_buffer().unwrap().saving);
            let token = app.editor_buffer().unwrap().token;
            let _ = app.update_editor(Event::Saved(
                token,
                if failed {
                    Err(Key::EditorConflict)
                } else {
                    Ok(())
                },
            ));
            assert_eq!(app.modules.deployment.busy, !failed);
            assert_eq!(app.editor.pending.is_some(), failed);
            if failed {
                assert_eq!(app.editor_buffer().unwrap().content.text(), "latest draft");
            } else {
                assert!(!app.modules.deployment.open);
            }
        }
    }
}

#[test]
fn uploading_folder_flushes_only_local_buffers_in_that_folder() {
    use crate::{
        app::modules::deployment_message,
        modules::deployment::{Event as DeploymentEvent, Operation},
    };
    let mut app = app();
    let local_id = app.preview().id;
    edit(&mut app, "local draft");
    let _ = app.open_remote_preview(
        "/srv/file.txt".into(),
        "/srv/file.txt".into(),
        ("profile".into(), "Server".into()),
        Arc::new("remote".into()),
        None,
    );
    let remote_id = app.preview().id;
    let _ = app.finish_preview(remote_id, Ok(Document::plain("remote".into())));
    edit(&mut app, "remote draft");
    let message = deployment_message(DeploymentEvent::Run(Operation::Upload("project".into())));
    assert!(app.protect_editor(&message));
    assert!(app.editor_pending_tab(local_id));
    assert!(!app.editor_pending_tab(remote_id));
}
