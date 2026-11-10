use super::*;
use crate::i18n::{I18n, Language};
use std::rc::Rc;

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
