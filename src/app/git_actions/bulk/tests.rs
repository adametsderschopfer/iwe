use super::*;
use crate::{
    app::git_actions,
    domain::repository::{ReferenceKind, Repository},
    i18n::{I18n, Language},
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    let branches = ["main", "one", "two"]
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
    app.branch_selection
        .selected
        .extend(["one", "two"].map(|name| BranchKey {
            root: "/project".into(),
            reference: format!("refs/heads/{name}"),
        }));
    app
}

#[test]
fn batches_require_confirmation_and_keep_source_identity_and_destination_names() {
    let mut app = app();
    assert!(app.bulk_allowed(Action::Push));
    let task = app.update_git(git_actions::Message::Bulk(Event::Choose(Action::Push)));
    assert_eq!(task.units(), 0);
    assert!(!app.git_actions.pending);
    let dialog = app.git_actions.bulk.dialog.as_ref().unwrap();
    assert_eq!(dialog.targets.len(), 2);
    for target in &dialog.targets {
        match target.operation(Action::Push) {
            Operation::Push {
                source,
                remote,
                name,
            } => {
                assert_eq!(source, target.key.reference);
                assert_eq!(remote, "origin");
                assert_eq!(name, target.branch.name);
            }
            _ => panic!("Expected a branch push"),
        }
    }
    assert!(app.update_bulk_git(Event::Submit).units() > 0);
    assert!(app.git_actions.pending);
    assert_eq!(app.update_bulk_git(Event::Submit).units(), 0);
}

#[test]
fn partial_failures_leave_only_failed_branches_for_an_explicit_retry() {
    let mut app = app();
    let _ = app.update_bulk_git(Event::Choose(Action::Delete));
    let targets = app
        .git_actions
        .bulk
        .dialog
        .as_ref()
        .unwrap()
        .targets
        .clone();
    let _ = app.update_bulk_git(Event::Submit);
    let request = app.git_actions.request;
    let generation = app.generation;
    let _ = app.update_bulk_git(Event::Finished(
        request,
        generation,
        vec![
            Outcome {
                target: targets[0].clone(),
                result: Ok(()),
            },
            Outcome {
                target: targets[1].clone(),
                result: Err(Error::BranchInWorktree),
            },
        ],
    ));
    assert!(!app.git_actions.pending);
    let dialog = app.git_actions.bulk.dialog.as_ref().unwrap();
    assert_eq!(dialog.targets.len(), 1);
    assert_eq!(dialog.targets[0].key, targets[1].key);
    assert_eq!(dialog.targets[0].error, Some(Error::BranchInWorktree));
    assert!(app.update_bulk_git(Event::Submit).units() > 0);
}

#[test]
fn current_branches_disable_batch_deletion_and_stale_completions_are_ignored() {
    let mut app = app();
    app.branch_selection.selected.insert(BranchKey {
        root: "/project".into(),
        reference: "refs/heads/main".into(),
    });
    assert!(!app.bulk_allowed(Action::Delete));
    assert_eq!(
        app.update_bulk_git(Event::Choose(Action::Delete)).units(),
        0
    );
    assert!(app.git_actions.bulk.dialog.is_none());
    app.branch_selection
        .selected
        .retain(|key| key.reference != "refs/heads/main");
    let _ = app.update_bulk_git(Event::Choose(Action::Pull));
    let _ = app.update_bulk_git(Event::Submit);
    let request = app.git_actions.request;
    let _ = app.update_bulk_git(Event::Finished(request + 1, app.generation, Vec::new()));
    assert!(app.git_actions.pending);
    assert!(app.git_actions.bulk.dialog.is_some());
    let _ = app.update_bulk_git(Event::Finished(request, app.generation + 1, Vec::new()));
    assert!(!app.git_actions.pending);
    assert!(app.git_actions.bulk.dialog.is_none());
}
