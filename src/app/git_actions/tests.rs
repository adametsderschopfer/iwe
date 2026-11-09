use super::*;
use crate::{
    domain::repository::Repository,
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.repository = Some(Repository {
        root: PathBuf::from("/test"),
        branches: vec![Branch {
            name: "main".into(),
            reference: "refs/heads/main".into(),
            current: true,
            kind: ReferenceKind::Local,
        }],
        commits: Vec::new(),
        next_history: None,
        remote: None,
        remotes: vec!["origin".into()],
        head: "main".into(),
    });
    app
}

#[test]
fn actions_validate_branch_capabilities_and_preserve_failed_drafts() {
    let mut app = app();
    let _ = app.update_git(Message::Choose(Action::Delete, "refs/heads/main".into()));
    assert!(app.git_actions.dialog.is_none());
    let _ = app.update_git(Message::Choose(Action::Create, "refs/heads/main".into()));
    let _ = app.update_git(Message::Name("topic".into()));
    let _ = app.update_git(Message::Submit);
    assert!(app.git_actions.pending);
    let request = app.git_actions.request;
    let _ = app.update_git(Message::Fetch);
    assert_eq!(app.git_actions.request, request);
    let _ = app.update_git(Message::Finished(
        request,
        PathBuf::from("/test"),
        Err(Error::Dirty),
    ));
    assert!(!app.git_actions.pending);
    let dialog = app.git_actions.dialog.as_ref().unwrap();
    assert_eq!(dialog.name, "topic");
    assert_eq!(dialog.error, Some(Error::Dirty));
}

#[test]
fn old_operation_results_cannot_replace_a_new_project() {
    let mut app = app();
    let _ = app.update_git(Message::Fetch);
    let request = app.git_actions.request;
    app.repository.as_mut().unwrap().root = PathBuf::from("/another");
    let generation = app.generation;
    let _ = app.update_git(Message::Finished(request, PathBuf::from("/test"), Ok(())));
    assert!(!app.git_actions.pending);
    assert_eq!(app.generation, generation);
    assert_eq!(app.repository.unwrap().root, PathBuf::from("/another"));
}

#[test]
fn branch_context_actions_and_translated_dialogs_fit_and_block_background_clicks() {
    use crate::{domain::repository::Commit, ui::theme};
    use iced::{Event, mouse, widget::Id};
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        app.workspace
            .create_space("Team".into(), vec![PathBuf::from("/test")]);
        app.workspace.preferences.show_git = true;
        app.branch = Some("refs/heads/main".into());
        app.repository.as_mut().unwrap().commits.push(Commit {
            id: "a".repeat(40),
            parents: vec![],
            subject: "Add branch tools".into(),
            author: "Test".into(),
            date: "2026-11-09".into(),
            refs: "HEAD -> main".into(),
            graph: Default::default(),
        });
        app.sync_panels();
        let settings = || iced::Settings {
            default_font: theme::font(),
            ..iced::Settings::default()
        };
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let branch = ui
            .find(Id::new("branch-refs/heads/main"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        ui.point_at(branch.center());
        ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Right,
        ))]);
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.git_actions.menu.is_some());
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("git-action-Delete")).unwrap();
        assert!(
            ui.into_messages().next().is_none(),
            "Current branch deletion must remain disabled"
        );
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("branch-menu-{suffix}")))
                .unwrap();
        }
        ui.click(Id::new("git-action-Create")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(
            app.git_actions.dialog.as_ref().unwrap().action,
            Action::Create
        );
        assert_eq!(
            app.git_actions.dialog.as_ref().unwrap().branch.reference,
            "refs/heads/main"
        );
        let _ = app.update_git(Message::Close);
        let name = format!("feature/{}", "long-branch-name-".repeat(12));
        let reference = format!("refs/heads/{name}");
        app.repository.as_mut().unwrap().branches.push(Branch {
            name,
            reference: reference.clone(),
            current: false,
            kind: ReferenceKind::Local,
        });
        for action in [Action::Create, Action::Rename, Action::Push, Action::Delete] {
            let _ = app.update_git(Message::Choose(action, reference.clone()));
            let _ = app.update_git(Message::Name("feature/new-name".into()));
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            let bounds = ui
                .find(Id::new("git-confirm"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(
                bounds.x >= 0.
                    && bounds.x + bounds.width <= 1080.
                    && bounds.y >= 0.
                    && bounds.y + bounds.height <= 720.
            );
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&iced::Theme::Dark)
                    .unwrap()
                    .matches_image(output.join(format!("git-{action:?}-{suffix}")))
                    .unwrap();
            }
            ui.click(Id::new("nav-settings")).unwrap();
            assert!(ui.into_messages().next().is_none());
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            ui.click(Id::new("git-confirm")).unwrap();
            assert!(matches!(
                ui.into_messages().collect::<Vec<_>>().as_slice(),
                [AppMessage::Git(Message::Submit)]
            ));
            let _ = app.update_git(Message::Close);
        }
    }
}
