use super::*;
use crate::{
    domain::workspace::{FileEntry, Project, Space},
    i18n::{I18n, Language},
};
use std::{collections::BTreeMap, path::PathBuf, rc::Rc};

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace.spaces = vec![Space {
        name: "Work".into(),
        projects: vec![
            Project {
                name: "Alpha".into(),
                directory: PathBuf::from("/tmp/alpha"),
                pinned: false,
            },
            Project {
                name: "Beta".into(),
                directory: PathBuf::from("/tmp/beta"),
                pinned: false,
            },
        ],
        deployments: Vec::new(),
    }];
    app.workspace.select_project(0, 0);
    let root = PathBuf::from("/tmp/alpha");
    let file = |path: PathBuf, name: &str| FileEntry {
        path,
        name: name.into(),
        directory: false,
    };
    app.tree = BTreeMap::from([
        (
            root.clone(),
            vec![
                file(root.join("main.rs"), "main.rs"),
                file(root.join("lib.rs"), "lib.rs"),
                FileEntry {
                    path: root.join("src"),
                    name: "src".into(),
                    directory: true,
                },
            ],
        ),
        (
            root.join("src"),
            vec![
                file(root.join("src/app.rs"), "app.rs"),
                file(root.join("src/mod.rs"), "mod.rs"),
            ],
        ),
    ]);
    app
}

#[test]
fn open_collects_files_and_refuses_other_dialogs_and_settings() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    assert!(app.search_everywhere.dialog.is_some());
    assert_eq!(app.search_everywhere.files.len(), 4);
    app.search_everywhere.dialog = None;

    app.navigation = Navigation::Settings;
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    assert!(app.search_everywhere.dialog.is_none());
    app.navigation = Navigation::Files;

    app.line_navigation = Some(crate::app::line_navigation::Dialog {
        id: 0,
        request: 0,
        value: String::new(),
        error: None,
        loading: false,
    });
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    assert!(app.search_everywhere.dialog.is_none());
}

#[test]
fn queries_rank_files_and_projects_into_rows() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    let _ = app.update_search_everywhere(Event::Changed("app".into()));
    let dialog = app.search_everywhere.dialog.as_ref().unwrap();
    assert_eq!(
        dialog.rows,
        vec![Row::File {
            name: "app.rs".into(),
            directory: "src".into(),
            path: PathBuf::from("/tmp/alpha/src/app.rs"),
        }]
    );

    let _ = app.update_search_everywhere(Event::Changed("al".into()));
    let dialog = app.search_everywhere.dialog.as_ref().unwrap();
    assert_eq!(
        dialog.rows,
        vec![Row::Project {
            space: 0,
            project: 0
        }]
    );

    let _ = app.update_search_everywhere(Event::Changed("zz".into()));
    assert!(
        app.search_everywhere
            .dialog
            .as_ref()
            .unwrap()
            .rows
            .is_empty()
    );
}

#[test]
fn scope_cycling_and_direct_changes_rebuild_rows() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    let _ = app.update_search_everywhere(Event::Changed("al".into()));
    let _ = app.update_search_everywhere(Event::CycleScope);
    let dialog = app.search_everywhere.dialog.as_ref().unwrap();
    assert_eq!(dialog.scope, Scope::Files);
    assert!(dialog.rows.is_empty());

    let _ = app.update_search_everywhere(Event::ChangeScope(Scope::Projects));
    let dialog = app.search_everywhere.dialog.as_ref().unwrap();
    assert_eq!(dialog.scope, Scope::Projects);
    assert_eq!(dialog.rows.len(), 1);
}

#[test]
fn selection_moves_and_wraps_through_rows() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    let _ = app.update_search_everywhere(Event::Changed("rs".into()));
    let length = app.search_everywhere.dialog.as_ref().unwrap().rows.len();
    assert!(length >= 2);
    let _ = app.update_search_everywhere(Event::Moved(false));
    assert_eq!(
        app.search_everywhere.dialog.as_ref().unwrap().selection,
        length - 1
    );
    for _ in 0..=length {
        let _ = app.update_search_everywhere(Event::Moved(true));
    }
    assert_eq!(app.search_everywhere.dialog.as_ref().unwrap().selection, 0);
}

#[test]
fn activating_a_file_row_closes_the_dialog_and_opens_the_file() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::Files));
    let _ = app.update_search_everywhere(Event::Changed("app".into()));
    let _ = app.update_search_everywhere(Event::Activate(0));
    assert!(app.search_everywhere.dialog.is_none());
    assert_eq!(app.tabs.files.len(), 1);
    assert_eq!(
        app.files.selected_path,
        Some(PathBuf::from("/tmp/alpha/src/app.rs"))
    );
}

#[test]
fn activating_a_project_row_switches_the_project() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    let _ = app.update_search_everywhere(Event::Changed("be".into()));
    let _ = app.update_search_everywhere(Event::Activate(0));
    assert!(app.search_everywhere.dialog.is_none());
    assert_eq!(app.workspace.selected_space, Some(0));
    assert_eq!(app.workspace.selected_project, Some(1));
}

#[test]
fn tree_changes_refresh_candidates_on_the_next_query() {
    let mut app = app();
    let _ = app.update_search_everywhere(Event::Open(Scope::All));
    let root = PathBuf::from("/tmp/alpha");
    app.tree.insert(
        root.join("docs"),
        vec![FileEntry {
            path: root.join("docs/guide.md"),
            name: "guide.md".into(),
            directory: false,
        }],
    );
    app.search_everywhere.tree_changed();
    let _ = app.update_search_everywhere(Event::Changed("guide".into()));
    let dialog = app.search_everywhere.dialog.as_ref().unwrap();
    assert_eq!(dialog.rows.len(), 1);
    assert!(matches!(&dialog.rows[0], Row::File { name, .. } if name == "guide.md"));
}

#[test]
fn visible_windows_center_the_selected_row() {
    let length = 30;
    assert_eq!(visible_window(length, 0), 0..theme::SEARCH_WINDOW_ROWS);
    assert_eq!(visible_window(length, 15), 12..20);
    assert_eq!(visible_window(length, 29), 22..30);
    assert_eq!(visible_window(3, 2), 0..3);
}
