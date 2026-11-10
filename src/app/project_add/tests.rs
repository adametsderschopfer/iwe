use super::*;
use crate::i18n::{I18n, Language};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app
}

#[test]
fn defaults_follow_the_target_space_and_fall_back_to_iwe_code() {
    let mut app = app();
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("work");
    app.workspace.create_space(
        "Work".into(),
        vec![
            work.join("api"),
            work.join("web"),
            root.path().join("other/one"),
        ],
    );
    app.workspace.create_space("Empty".into(), vec![]);
    let _ = app.update_project_add(Message::Open(Some(0)));
    let _ = app.update_project_add(Message::Remote);
    assert_eq!(
        app.project_add.as_ref().unwrap().destination,
        work.to_string_lossy()
    );
    let _ = app.update_project_add(Message::Url("git@example.com:team/mobile.git".into()));
    assert_eq!(
        app.project_add.as_ref().unwrap().destination,
        work.join("mobile").to_string_lossy()
    );
    let _ = app.update_project_add(Message::Url(String::new()));
    assert_eq!(
        app.project_add.as_ref().unwrap().destination,
        work.to_string_lossy()
    );
    let _ = app.update_project_add(Message::PickParent);
    let _ = app.update_project_add(Message::ParentPicked(
        app.space_request,
        Some(root.path().to_owned()),
    ));
    assert_eq!(
        app.project_add.as_ref().unwrap().destination,
        root.path().to_string_lossy()
    );
    let _ = app.update_project_add(Message::Cancel);
    let _ = app.update_project_add(Message::Open(Some(1)));
    assert_eq!(
        app.project_add.as_ref().unwrap().parent,
        project_clone::default_parent()
    );
    assert_eq!(
        app.project_add.as_ref().unwrap().destination,
        project_clone::default_parent().to_string_lossy()
    );
}

#[test]
fn custom_paths_survive_url_edits_and_failures_and_stale_results_are_ignored() {
    let mut app = app();
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("location");
    app.workspace.create_space("Work".into(), vec![]);
    let _ = app.update_project_add(Message::Open(Some(0)));
    let _ = app.update_project_add(Message::Remote);
    let _ = app.update_project_add(Message::Destination(
        destination.to_string_lossy().into_owned(),
    ));
    let _ = app.update_project_add(Message::Url("https://example.com/a.git".into()));
    let _ = app.update_project_add(Message::Clone);
    let request = app.space_request;
    let _ = app.update_project_add(Message::Cancel);
    assert!(app.project_add.as_ref().unwrap().pending);
    let _ = app.update_project_add(Message::Finished(request - 1, Ok("/stale".into())));
    assert!(app.workspace.spaces[0].projects.is_empty());
    let _ = app.update_project_add(Message::Finished(request, Err(Error::Failed)));
    let draft = app.project_add.as_ref().unwrap();
    assert_eq!(draft.destination, destination.to_string_lossy());
    assert_eq!(draft.error, Some(Error::Failed));
    let _ = app.update_project_add(Message::Clone);
    let _ = app.update_project_add(Message::Finished(request, Ok("/obsolete".into())));
    assert!(app.workspace.spaces[0].projects.is_empty());
    let _ = app.update_project_add(Message::Finished(
        app.space_request,
        Ok(destination.clone()),
    ));
    assert!(app.project_add.is_none());
    assert_eq!(app.workspace.directory(), Some(&destination));
}

#[test]
fn adding_to_a_space_form_only_updates_its_draft() {
    let mut app = app();
    let root = tempfile::tempdir().unwrap();
    let _ = app.update_space(spaces::Message::New);
    app.space_draft
        .as_mut()
        .unwrap()
        .directories
        .push(root.path().join("existing"));
    let _ = app.update_project_add(Message::Open(None));
    assert_eq!(app.project_add.as_ref().unwrap().parent, root.path());
    let _ = app.update_project_add(Message::Remote);
    let _ = app.update_project_add(Message::Url("https://example.com/a.git".into()));
    let _ = app.update_project_add(Message::Clone);
    let _ = app.update_project_add(Message::Finished(app.space_request, Ok("/cloned/a".into())));
    assert!(app.workspace.spaces.is_empty());
    assert_eq!(
        app.space_draft.as_ref().unwrap().directories,
        [root.path().join("existing"), PathBuf::from("/cloned/a")]
    );
    let _ = app.update_space(spaces::Message::Cancel);
    assert!(app.workspace.spaces.is_empty());
}

#[test]
fn translated_dialogs_fit_and_block_background_actions() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        app.workspace.create_space("Work".into(), vec![]);
        let _ = app.update_project_add(Message::Open(Some(0)));
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
        ui.click(Id::new("nav-settings")).unwrap();
        assert!(ui.into_messages().next().is_none());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1080., 720.),
                app.view(),
            );
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("project-source-{language:?}")))
                .unwrap();
        }
        let _ = app.update_project_add(Message::Remote);
        let _ = app.update_project_add(Message::Url(
            "https://example.com/team/repository.git".into(),
        ));
        app.project_add.as_mut().unwrap().error = Some(Error::Failed);
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            for (theme, appearance) in [(iced::Theme::Dark, "dark"), (iced::Theme::Light, "light")]
            {
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings::default(),
                    (1080., 720.),
                    app.view(),
                );
                ui.snapshot(&theme)
                    .unwrap()
                    .matches_image(output.join(format!("clone-{language:?}-{appearance}")))
                    .unwrap();
            }
        }
        for id in [
            "clone-url",
            "clone-destination",
            "clone-submit",
            "clone-pick-parent",
            "add-project-cancel",
        ] {
            let mut ui = iced_test::Simulator::with_size(
                iced::Settings::default(),
                (1080., 720.),
                app.view(),
            );
            let bounds = ui.find(Id::new(id)).unwrap().visible_bounds().unwrap();
            assert!(
                bounds.x >= 0.
                    && bounds.y >= 0.
                    && bounds.x + bounds.width <= 1080.
                    && bounds.y + bounds.height <= 720.
            );
        }
    }
}

#[test]
fn ambiguous_parents_use_the_fallback_and_canceled_pickers_cannot_change_new_dialogs() {
    let mut app = app();
    let root = tempfile::tempdir().unwrap();
    app.workspace.create_space(
        "Work".into(),
        vec![root.path().join("one/api"), root.path().join("two/web")],
    );
    let _ = app.update_project_add(Message::Open(Some(0)));
    assert_eq!(
        app.project_add.as_ref().unwrap().parent,
        project_clone::default_parent()
    );
    let request = app.space_request;
    let _ = app.update_project_add(Message::PickParent);
    let _ = app.update_project_add(Message::Cancel);
    let _ = app.update_project_add(Message::Open(Some(0)));
    let _ = app.update_project_add(Message::ParentPicked(request, Some(root.path().to_owned())));
    assert_eq!(
        app.project_add.as_ref().unwrap().parent,
        project_clone::default_parent()
    );
}
