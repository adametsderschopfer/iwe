use super::*;
use crate::{
    domain::repository::Repository,
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn branch_key(reference: impl Into<String>) -> crate::domain::repository::project::BranchKey {
    crate::domain::repository::project::BranchKey {
        root: PathBuf::from("/test"),
        reference: reference.into(),
    }
}

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.repository = Some(
        Repository {
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
        }
        .into(),
    );
    app
}

#[test]
fn scoped_dialog_uses_its_root_even_when_another_repository_is_active() {
    let mut app = app();
    let repos = app.repository.as_mut().unwrap();
    let mut other = repos.repositories[0].clone();
    other.root = PathBuf::from("/front");
    repos.repositories.push(other);
    repos.active_root = PathBuf::from("/front");
    let _ = app.update_git(Message::Choose(
        Action::Create,
        branch_key("refs/heads/main"),
    ));
    let _ = app.update_git(Message::Name("topic".into()));
    assert_eq!(
        app.git_actions.dialog.as_ref().unwrap().root,
        PathBuf::from("/test")
    );
    let _ = app.update_git(Message::Submit);
    let request = app.git_actions.request;
    let _ = app.update_git(Message::Finished(
        request,
        PathBuf::from("/test"),
        Err(Error::Dirty),
    ));
    assert_eq!(
        app.git_actions.dialog.as_ref().unwrap().error,
        Some(Error::Dirty)
    );
    assert_eq!(
        app.active_repository().unwrap().root,
        PathBuf::from("/front")
    );
    assert_eq!(app.notifications.items.len(), 1);
}

#[test]
fn fetch_reports_each_repository_and_ignores_duplicate_or_obsolete_results() {
    for obsolete in [false, true] {
        let mut app = app();
        let repos = app.repository.as_mut().unwrap();
        let mut other = repos.repositories[0].clone();
        other.root = PathBuf::from("/front");
        repos.repositories.push(other);
        let generation = app.generation;
        let _ = app.update_git(Message::Fetch);
        let request = app.git_actions.request;
        assert!(app.git_actions.pending);
        if obsolete {
            app.generation += 1;
        }
        let message = Message::Fetched(
            request,
            generation,
            vec![
                (PathBuf::from("/test"), Ok(())),
                (PathBuf::from("/front"), Err(Error::Failed)),
            ],
        );
        let _ = app.update_git(message.clone());
        assert!(!app.git_actions.pending);
        assert_eq!(app.notifications.items.len(), if obsolete { 0 } else { 2 });
        let _ = app.update_git(message);
        assert_eq!(app.notifications.items.len(), if obsolete { 0 } else { 2 });
    }
}

#[test]
fn actions_validate_branch_capabilities_and_preserve_failed_drafts() {
    let mut app = app();
    let _ = app.update_git(Message::Choose(
        Action::Delete,
        branch_key("refs/heads/main"),
    ));
    assert!(app.git_actions.dialog.is_none());
    let _ = app.update_git(Message::Choose(
        Action::Create,
        branch_key("refs/heads/main"),
    ));
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
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    ui.click(iced::widget::Id::new("notification-close-1"))
        .unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert!(app.notifications.items.is_empty());
    assert!(app.git_actions.dialog.is_some());
}

#[test]
fn old_operation_results_cannot_replace_a_new_project() {
    let mut app = app();
    let _ = app.update_git(Message::Fetch);
    let request = app.git_actions.request;
    app.repository.as_mut().unwrap().repositories[0].root = PathBuf::from("/another");
    let generation = app.generation;
    let _ = app.update_git(Message::Finished(request, PathBuf::from("/test"), Ok(())));
    assert!(!app.git_actions.pending);
    assert_eq!(app.generation, generation);
    assert_eq!(
        app.repository.unwrap().repositories[0].root,
        PathBuf::from("/another")
    );
    assert!(app.notifications.items.is_empty());
}

#[test]
fn git_results_notify_once_after_completion_and_keep_failures_visible() {
    for (operation, key) in [
        (Operation::Fetch, Key::GitFetchDone),
        (
            Operation::Checkout("refs/heads/main".into()),
            Key::GitCheckoutDone,
        ),
        (
            Operation::Update("refs/heads/main".into()),
            Key::GitUpdateDone,
        ),
        (
            Operation::Create {
                source: "refs/heads/main".into(),
                name: "topic".into(),
                checkout: false,
            },
            Key::GitCreateDone,
        ),
        (
            Operation::Rename {
                source: "refs/heads/main".into(),
                name: "topic".into(),
            },
            Key::GitRenameDone,
        ),
        (
            Operation::Push {
                source: "refs/heads/main".into(),
                remote: "origin".into(),
                name: "main".into(),
            },
            Key::GitPushDone,
        ),
        (
            Operation::Delete("refs/heads/topic".into()),
            Key::GitDeleteDone,
        ),
    ] {
        let mut app = app();
        app.notifications = Default::default();
        let _ = app.run_git(PathBuf::from("/test"), operation);
        assert!(app.notifications.items.is_empty());
        let request = app.git_actions.request;
        let _ = app.update_git(Message::Finished(request, "/test".into(), Ok(())));
        assert_eq!(app.notifications.items.len(), 1);
        assert_eq!(app.notifications.items[0].message, key);
        let _ = app.update_git(Message::Finished(request, "/test".into(), Ok(())));
        assert_eq!(app.notifications.items.len(), 1);
    }
    let mut app = app();
    app.notifications = Default::default();
    let _ = app.run_git(PathBuf::from("/test"), Operation::Fetch);
    let _ = app.update_git(Message::Finished(
        app.git_actions.request,
        "/test".into(),
        Err(Error::Failed),
    ));
    assert_eq!(
        app.notifications.items[0].kind,
        crate::app::notifications::Kind::Error
    );
    assert_eq!(app.notifications.items[0].message, Key::GitOperationFailed);
}

#[test]
fn long_branch_and_folder_names_remain_on_one_line() {
    use crate::{domain::repository::Commit, ui::theme};
    use iced::widget::Id;
    let mut app = app();
    app.workspace
        .create_space("Work".into(), vec!["/test".into()]);
    let current = format!("feature/{}", "длинное-название-".repeat(8));
    let folder = "длинное-название-группы-".repeat(8);
    let nested = format!("{folder}/{}", "nested-branch-".repeat(8));
    let repo = app.repository.as_mut().unwrap();
    repo.active_mut().unwrap().branches[0].name = current.clone();
    repo.active_mut().unwrap().branches[0].reference = format!("refs/heads/{current}");
    repo.active_mut().unwrap().branches.push(Branch {
        name: nested.clone(),
        reference: format!("refs/heads/{nested}"),
        current: false,
        kind: ReferenceKind::Local,
    });
    repo.commits
        .push(crate::domain::repository::project::ScopedCommit {
            root: PathBuf::from("/test"),
            commit: Commit {
                id: "a".repeat(40),
                parents: vec![],
                subject: "Preview".into(),
                author: "Test".into(),
                date: "2026-11-09".into(),
                timestamp: 0,
                refs: String::new(),
                graph: Default::default(),
            },
        });
    app.expanded_branch_folders
        .insert(format!("false/{folder}"));
    app.workspace.preferences.show_git = true;
    app.sync_panels();
    for width in [1080., 1440.] {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: theme::font(),
                ..Default::default()
            },
            (width, 920.),
            app.view(),
        );
        let panel = ui
            .find(Id::new("git-branches"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        for name in [&current, &nested] {
            let bounds = ui
                .find(Id::from(format!("branch-refs/heads/{name}")))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert_eq!(bounds.height, 30.);
            assert!(bounds.x + bounds.width <= panel.x + panel.width);
            assert!(ui.find(name.as_str()).is_err());
        }
        let list = ui
            .find(Id::new("git-branch-scroll"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!((list.x + list.width - panel.x - panel.width).abs() < 1.);
        let mut previous = None::<iced::Rectangle>;
        for id in ["git-fetch", "git-new-branch", "git-update", "git-push"] {
            let bounds = ui.find(Id::new(id)).unwrap().visible_bounds().unwrap();
            assert!(bounds.y + bounds.height <= list.y);
            assert!(bounds.x >= panel.x && bounds.x + bounds.width <= panel.x + panel.width);
            if let Some(previous) = previous {
                assert_eq!(bounds.y, previous.y);
                assert_eq!(bounds.height, previous.height);
                assert!(bounds.x > previous.x + previous.width);
            }
            previous = Some(bounds);
        }
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("single-line-branches-{width}")))
                .unwrap();
        }
        ui.click(Id::from(format!("branch-refs/heads/{nested}")))
            .unwrap();
        assert!(ui.into_messages().any(|message| matches!(message, AppMessage::SelectBranch(reference) if reference.reference == format!("refs/heads/{nested}"))));
    }
}

#[test]
fn branch_toolbar_and_scrollbar_fit_the_minimum_pane_width() {
    use crate::{domain::repository::Commit, ui::theme};
    use iced::{
        Event, mouse,
        widget::{Id, pane_grid},
    };
    let mut app = app();
    app.workspace
        .create_space("Work".into(), vec!["/test".into()]);
    app.branch = Some(branch_key("refs/heads/main"));
    let repo = app.repository.as_mut().unwrap();
    repo.commits
        .push(crate::domain::repository::project::ScopedCommit {
            root: PathBuf::from("/test"),
            commit: Commit {
                id: "a".repeat(40),
                parents: vec![],
                subject: "Preview".into(),
                author: "Test".into(),
                date: "2026-11-09".into(),
                timestamp: 0,
                refs: String::new(),
                graph: Default::default(),
            },
        });
    for index in 0..60 {
        repo.active_mut().unwrap().branches.push(Branch {
            name: format!("branch-{index:02}"),
            reference: format!("refs/heads/branch-{index:02}"),
            current: false,
            kind: ReferenceKind::Local,
        });
    }
    if let pane_grid::Node::Split { id, .. } = app.git_panes.layout() {
        app.git_panes.resize(*id, 0.01);
    }
    app.workspace.preferences.show_git = true;
    app.sync_panels();
    for appearance in [
        crate::domain::workspace::Appearance::Dark,
        crate::domain::workspace::Appearance::Light,
    ] {
        app.workspace.preferences.appearance = appearance;
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: theme::font(),
                ..Default::default()
            },
            (1080., 720.),
            app.view(),
        );
        let panel = ui
            .find(Id::new("git-branches"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!((panel.width - theme::GIT_PANE_MIN_SIZE).abs() < 1.);
        let list = ui
            .find(Id::new("git-branch-scroll"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!((list.x + list.width - panel.x - panel.width).abs() < 1.);
        let first = ui
            .find(Id::new("branch-refs/heads/main"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        for id in ["git-fetch", "git-new-branch", "git-update", "git-push"] {
            let bounds = ui.find(Id::new(id)).unwrap().visible_bounds().unwrap();
            assert!(bounds.width >= 20.);
            assert!(bounds.x >= panel.x && bounds.x + bounds.width <= panel.x + panel.width);
            assert!(bounds.y + bounds.height <= first.y);
        }
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&app.theme())
                .unwrap()
                .matches_image(output.join(format!("branch-toolbar-minimum-{appearance:?}")))
                .unwrap();
        }
        let fetch = ui
            .find(Id::new("git-fetch"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        ui.point_at((list.x + list.width - 3., list.y + 8.));
        ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y: -120. },
        })]);
        assert_eq!(
            ui.find(Id::new("git-fetch"))
                .unwrap()
                .visible_bounds()
                .unwrap(),
            fetch
        );
        assert!(
            ui.find(Id::new("branch-refs/heads/main"))
                .unwrap()
                .visible_bounds()
                .is_none()
        );
        ui.click(Id::new("git-fetch")).unwrap();
        assert!(
            ui.into_messages()
                .any(|message| matches!(message, AppMessage::Git(Message::Fetch)))
        );
    }
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
        app.branch = Some(branch_key("refs/heads/main"));
        app.repository.as_mut().unwrap().commits.push(
            crate::domain::repository::project::ScopedCommit {
                root: PathBuf::from("/test"),
                commit: Commit {
                    id: "a".repeat(40),
                    parents: vec![],
                    subject: "Add branch tools".into(),
                    author: "Test".into(),
                    date: "2026-11-09".into(),
                    timestamp: 0,
                    refs: "HEAD -> main".into(),
                    graph: Default::default(),
                },
            },
        );
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
        app.repository
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .branches
            .push(Branch {
                name,
                reference: reference.clone(),
                current: false,
                kind: ReferenceKind::Local,
            });
        for action in [Action::Create, Action::Rename, Action::Push, Action::Delete] {
            let _ = app.update_git(Message::Choose(action, branch_key(reference.clone())));
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
