use super::*;
use crate::domain::repository::{ReferenceKind, project::BranchKey};
use crate::{
    app::git_actions,
    domain::repository::{Branch, Repository},
    i18n::{I18n, Key, Language},
};
use iced::Point;
use std::collections::BTreeSet;
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec!["/project".into()]);
    let branches = ["main", "feature/a", "feature/nested/b", "featurex/c"]
        .into_iter()
        .map(|name| Branch {
            name: name.into(),
            reference: format!("refs/heads/{name}"),
            current: name == "main",
            kind: ReferenceKind::Local,
        })
        .collect();
    app.repository = Some(
        Repository {
            root: "/project".into(),
            branches,
            commits: Vec::new(),
            next_history: None,
            remote: None,
            remotes: vec!["origin".into()],
            head: "main".into(),
        }
        .into(),
    );
    app.panes = iced::widget::pane_grid::State::new(crate::app::Panel::Module(
        crate::modules::ModuleId::Git,
    ))
    .0;
    app
}

fn key(name: &str) -> BranchKey {
    BranchKey {
        root: "/project".into(),
        reference: format!("refs/heads/{name}"),
    }
}
fn folder() -> Target {
    Target::Group {
        root: "/project".into(),
        kind: Some(ReferenceKind::Local),
        prefix: Some("feature/".into()),
    }
}

#[test]
fn folders_toggle_unique_descendants_without_changing_the_history_filter() {
    let mut app = app();
    app.modules.git.branch = Some(key("main"));
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    assert_eq!(
        app.modules.git.branch_selection.selected,
        BTreeSet::from([key("feature/a"), key("feature/nested/b")])
    );
    let _ = app.update_branch_selection(Event::Toggle(Target::Branch(key("feature/a"))));
    assert_eq!(app.modules.git.branch_selection.selected.len(), 1);
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    assert_eq!(app.modules.git.branch_selection.selected.len(), 2);
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    assert!(app.modules.git.branch_selection.selected.is_empty());
    assert_eq!(app.modules.git.branch, Some(key("main")));
}

#[test]
fn normal_clicks_and_outside_clicks_clear_selection_but_command_clicks_preserve_it() {
    use iced::widget::Id;
    let mut app = app();
    app.modules.git.branch_selection.modifiers = keyboard::Modifiers::COMMAND;
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1440., 1000.), app.view());
    ui.click("feature").unwrap();
    let messages: Vec<_> = ui
        .into_messages()
        .map(crate::app::modules::host_message)
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::BranchSelection(Event::Toggle(_))))
    );
    assert!(!messages.iter().any(|message| matches!(
        message,
        Message::ToggleBranchFolder(_) | Message::BranchSelection(Event::Clear)
    )));
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(app.modules.git.branch_selection.selected.len(), 2);
    assert!(
        !app.modules
            .git
            .expanded_branch_folders
            .contains("false/feature")
    );
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1440., 1000.), app.view());
    assert!(ui.find(Id::new("branch-selection-count")).is_ok());
    ui.click(app.i18n.text(Key::Branches)).unwrap();
    for message in ui.into_messages().map(crate::app::modules::host_message) {
        let _ = app.update(message);
    }
    assert!(app.modules.git.branch_selection.selected.is_empty());
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    app.modules.git.branch_selection.modifiers = keyboard::Modifiers::empty();
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1440., 1000.), app.view());
    ui.click(Id::new("branch-refs/heads/main")).unwrap();
    for message in ui.into_messages().map(crate::app::modules::host_message) {
        let _ = app.update(message);
    }
    assert!(app.modules.git.branch_selection.selected.is_empty());
    assert_eq!(app.modules.git.branch, Some(key("main")));
}

#[test]
fn selection_menu_has_only_bulk_actions_and_ignores_other_repositories_with_identical_refs() {
    let mut app = app();
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    let _ = app.update_git(git_actions::Message::Menu(
        key("feature/a"),
        Point::new(300., 200.),
    ));
    assert!(app.modules.git.git_actions.menu.is_none());
    assert!(app.modules.git.git_actions.bulk.menu.is_some());
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1440., 1000.), app.view());
    for action in ["Push", "Delete", "Pull"] {
        assert!(
            ui.find(iced::widget::Id::from(format!("git-bulk-{action}")))
                .is_ok()
        );
    }
    for action in ["Checkout", "Create", "Rename"] {
        assert!(
            ui.find(iced::widget::Id::from(format!("git-action-{action}")))
                .is_err()
        );
    }
    let other = BranchKey {
        root: "/other".into(),
        reference: key("feature/a").reference,
    };
    assert!(!app.modules.git.branch_selection.selected.contains(&other));
    drop(ui);
    let _ = app.update_branch_selection(Event::Clear);
    assert!(app.modules.git.git_actions.bulk.menu.is_none());
}

#[test]
fn refresh_prunes_deleted_branch_identities() {
    let mut app = app();
    let _ = app.update_branch_selection(Event::Toggle(folder()));
    let repos = app.repository.as_mut().unwrap();
    repos.repositories[0]
        .branches
        .retain(|branch| branch.name != "feature/a");
    app.modules.git.branch_selection.prune(repos);
    assert_eq!(
        app.modules.git.branch_selection.selected,
        BTreeSet::from([key("feature/nested/b")])
    );
}
