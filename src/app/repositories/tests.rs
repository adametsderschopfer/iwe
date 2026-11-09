use super::*;
use crate::{
    app::{
        GitPanel,
        git_actions::{Action, Message as GitMessage},
    },
    domain::{
        changes::{FileChange, FileStatus, Worktree},
        repository::{
            Branch, ChangedFile, Commit, GraphRow, ReferenceKind,
            project::{BranchKey, ScopedCommit},
        },
        workspace::FilePanel,
    },
    i18n::{I18n, Language},
    ui::{code::Document, theme},
};
use iced::{
    Theme,
    widget::{Id, pane_grid},
};
use std::{collections::BTreeMap, path::PathBuf, rc::Rc};

fn root(name: &str) -> PathBuf {
    PathBuf::from("project").join(name)
}
fn branch(name: &str) -> BranchKey {
    BranchKey {
        root: root(name),
        reference: "refs/heads/main".into(),
    }
}
fn key(name: &str) -> CommitKey {
    CommitKey {
        root: root(name),
        id: "a".repeat(40),
    }
}

fn fixture() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.workspace
        .create_space("Company website".into(), vec!["project".into()]);
    let repositories = ["backend", "front"]
        .map(|name| Repository {
            root: root(name),
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
        })
        .to_vec();
    let commits = ["backend", "front"]
        .map(|name| ScopedCommit {
            root: root(name),
            commit: Commit {
                id: key(name).id,
                parents: vec![],
                subject: format!("{name} change"),
                author: "Author".into(),
                date: "2026-11-09 16:30".into(),
                timestamp: 1_700_000_000,
                refs: "HEAD -> main".into(),
                graph: GraphRow::default(),
            },
        })
        .to_vec();
    let repos = RepositorySet {
        repositories,
        active_root: root("backend"),
        commits,
        next_history: None,
        failures: Vec::new(),
    };
    let _ = app.update(Message::RepositoryLoaded(0, 0, Ok(repos)));
    app.changes.loaded = true;
    app.changes.status = Worktree::new(
        &PathBuf::from("project"),
        BTreeMap::from([
            (
                root("backend").join("src/same.rs"),
                FileChange {
                    status: FileStatus::Modified,
                    original: None,
                },
            ),
            (
                root("front").join("src/same.rs"),
                FileChange {
                    status: FileStatus::Added,
                    original: None,
                },
            ),
        ]),
    );
    app.changes.status.roots = vec![root("backend"), root("front")];
    app.file_panel = FilePanel::Changes;
    app
}

#[test]
fn branch_and_commit_selection_keep_the_repository_identity() {
    let mut app = fixture();
    assert!(app.branch.is_none());
    let _ = app.update(Message::SelectBranch(branch("front")));
    assert_eq!(app.branch, Some(branch("front")));
    let _ = app.update_git(GitMessage::Menu(branch("backend"), iced::Point::ORIGIN));
    assert_eq!(
        app.git_actions.menu.as_ref().unwrap().0.root,
        root("backend")
    );
    let _ = app.update_git(GitMessage::Choose(Action::Create, branch("backend")));
    assert_eq!(
        app.git_actions.dialog.as_ref().unwrap().root,
        root("backend")
    );
    let _ = app.update_git(GitMessage::Close);
    let _ = app.update(Message::SelectCommit(key("front")));
    let old_request = app.commit_request;
    let _ = app.update(Message::ChangesLoaded(
        0,
        old_request,
        Ok(vec![ChangedFile {
            path: "same.rs".into(),
            status: 'M',
        }]),
    ));
    let _ = app.open_commit_file(0);
    assert_eq!(
        app.preview().path.as_ref(),
        Some(&root("front").join("same.rs"))
    );
    let _ = app.update(Message::SelectCommit(key("backend")));
    let request = app.commit_request;
    let _ = app.update(Message::ChangesLoaded(
        0,
        old_request,
        Ok(vec![ChangedFile {
            path: "wrong.rs".into(),
            status: 'A',
        }]),
    ));
    assert!(app.changed_files.is_empty());
    let _ = app.update(Message::ChangesLoaded(
        0,
        request,
        Ok(vec![ChangedFile {
            path: "same.rs".into(),
            status: 'M',
        }]),
    ));
    let _ = app.open_commit_file(0);
    assert_eq!(
        app.preview().path.as_ref(),
        Some(&root("backend").join("same.rs"))
    );
    assert_eq!(app.tabs.files.len(), 2);
    let _ = app.update(Message::AllRepositories);
    assert!(app.branch.is_none());
    assert!(app.selected_commit.is_none());
}

#[test]
fn grouped_branches_changes_and_commit_badges_show_both_repositories() {
    let mut app = fixture();
    let _ = app.open_file(root("front").join("src/same.rs"));
    let _ = app.finish_preview(
        app.preview().id,
        Ok(Document::new("fn main() {}\n".into(), "rs")),
    );
    app.workspace.preferences.file_panel_width = 330.;
    let settings = || iced::Settings {
        default_font: theme::font(),
        ..iced::Settings::default()
    };
    for language in [Language::English, Language::Russian, Language::Spanish] {
        app.i18n = Rc::new(I18n::new(language));
        let mut ui = iced_test::Simulator::with_size(settings(), (1440., 1000.), app.view());
        for name in ["backend", "front"] {
            assert!(
                ui.find(Id::from(format!(
                    "changes-repository-{}",
                    root(name).display()
                )))
                .is_ok()
            );
            assert!(
                ui.find(Id::from(format!(
                    "branch-{}-refs/heads/main",
                    root(name).display()
                )))
                .is_ok()
            );
        }
        if language == Language::Russian
            && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
        {
            for (theme, suffix) in [(Theme::Dark, "dark"), (Theme::Light, "light")] {
                ui.snapshot(&theme)
                    .unwrap()
                    .matches_image(output.join(format!("nested-repositories-{suffix}")))
                    .unwrap();
            }
        }
        ui.click(Id::from(format!(
            "branch-{}-refs/heads/main",
            root("front").display()
        )))
        .unwrap();
        assert!(ui.into_messages().any(
            |message| matches!(message, Message::SelectBranch(key) if key == branch("front"))
        ));
        let mut ui = iced_test::Simulator::with_size(settings(), (1440., 1000.), app.view());
        ui.click("front change").unwrap();
        assert!(ui.into_messages().any(
            |message| matches!(message, Message::SelectCommit(selected) if selected == key("front"))
        ));
    }
}

#[test]
fn empty_history_keeps_repository_branches_visible() {
    let mut app = fixture();
    app.repository.as_mut().unwrap().commits.clear();
    app.panes = pane_grid::State::new(crate::app::Panel::Git).0;
    app.git_panes = super::super::git_panes();
    assert!(
        app.git_panes
            .iter()
            .any(|(_, panel)| matches!(panel, GitPanel::Branches))
    );
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    assert!(
        ui.find(Id::from(format!(
            "branch-{}-refs/heads/main",
            root("front").display()
        )))
        .is_ok()
    );
    assert!(ui.find(app.i18n.text(crate::i18n::Key::NoCommits)).is_ok());
}

#[test]
fn refresh_expands_repository_groups_but_filter_changes_preserve_them() {
    let mut app = fixture();
    app.project_directory = app.workspace.directory().cloned();
    let group = format!("repo:{}", root("front").display());
    let local = format!("{}:false", root("front").display());
    let _ = app.update(Message::ToggleBranchFolder(group.clone()));
    let _ = app.update(Message::SelectBranch(branch("backend")));
    let repos = app.repository.clone().unwrap();
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Ok(repos.clone()),
    ));
    assert!(!app.expanded_branch_folders.contains(&group));
    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Ok(repos),
    ));
    assert!(app.expanded_branch_folders.contains(&group));
    assert!(app.expanded_branch_folders.contains(&local));
}

#[test]
fn discovering_another_repository_keeps_both_branch_groups_visible() {
    let mut app = fixture();
    let repos = app.repository.clone().unwrap();
    app.repository.as_mut().unwrap().repositories.truncate(1);
    app.expanded_branch_folders = std::collections::BTreeSet::from(["false".into()]);
    let _ = app.update(Message::RepositoryLoaded(0, 0, Ok(repos)));
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    for name in ["backend", "front"] {
        assert!(
            ui.find(Id::from(format!(
                "branch-{}-refs/heads/main",
                root(name).display()
            )))
            .is_ok()
        );
    }
}
