use super::*;
use crate::i18n::{I18n, Language};
use std::rc::Rc;

#[test]
fn menu_accelerators_pause_only_while_recording_shortcuts() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    assert!(!app.menu_model().recording_shortcut);
    let _ = app.update_shortcuts(super::super::shortcuts::Event::Record(
        crate::domain::shortcuts::Command::Copy,
    ));
    assert!(app.menu_model().recording_shortcut);
    let _ = app.update_shortcuts(super::super::shortcuts::Event::Cancel);
    assert!(!app.menu_model().recording_shortcut);
}

#[test]
fn menu_model_tracks_spaces_projects_and_languages() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.loaded = true;
        app.workspace
            .create_space("Work".into(), vec!["/front".into(), "/back".into()]);
        let model = app.menu_model();
        assert_eq!(model.settings, app.i18n.text(Key::Settings));
        assert_eq!(model.new_space, app.i18n.text(Key::NewSpace));
        assert_eq!(model.spaces[0].0, "Work");
        assert_eq!(model.spaces[0].1[0].0, "front");
        assert_eq!(model.spaces[0].1[1].0, "back");
        assert!(model.enabled);
        let _ = app.update_space(spaces::Message::New);
        assert!(!app.menu_model().enabled);
    }
}

#[test]
fn new_space_menu_action_opens_creation_and_respects_modal_dialogs() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace.create_space("Work".into(), vec![]);
    let _ = app.update_native_menu(Event::Polled(Ok(vec![Action::NewSpace])));
    assert!(
        app.space_draft
            .as_ref()
            .is_some_and(|draft| draft.editing.is_none())
    );
    assert_eq!(app.workspace.spaces.len(), 1);
    assert!(!app.menu_model().enabled);
    let _ = app.update_native_menu(Event::Polled(Ok(vec![Action::NewSpace])));
    assert_eq!(app.workspace.spaces.len(), 1);
    assert!(
        app.space_draft
            .as_ref()
            .is_some_and(|draft| draft.editing.is_none())
    );
}

#[test]
fn menu_project_actions_resolve_paths_after_reordering() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec!["/front".into(), "/back".into()]);
    app.workspace.move_project(0, 0, 1);
    let _ = app.update_native_menu(Event::Polled(Ok(vec![Action::Project(0, "/front".into())])));
    assert_eq!(
        app.workspace.directory(),
        Some(&std::path::PathBuf::from("/front"))
    );
    let _ = app.update_native_menu(Event::Polled(Ok(vec![Action::Project(
        0,
        "/removed".into(),
    )])));
    assert_eq!(
        app.workspace.directory(),
        Some(&std::path::PathBuf::from("/front"))
    );
}

#[test]
fn deployment_menu_opens_matching_settings_and_preserves_project() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec!["/front".into()]);
    let project = app.workspace.directory().cloned();
    for (action, tab) in [
        (Action::DeploymentConnections, SettingsTab::Connections),
        (Action::DeploymentKeys, SettingsTab::DeploymentKeys),
        (Action::DeploymentProfiles, SettingsTab::Deployment),
    ] {
        let _ = app.update_native_menu(Event::Polled(Ok(vec![action])));
        assert_eq!(app.settings_tab, tab);
        assert!(app.windows.settings.is_some());
        assert_eq!(app.workspace.directory(), project.as_ref());
    }
}
