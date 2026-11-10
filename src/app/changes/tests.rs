use super::*;
use crate::i18n::{I18n, Language};
use std::{path::PathBuf, rc::Rc};

fn folder_test_app(grouped: bool) -> (App, Vec<PathBuf>) {
    use crate::domain::{
        changes::{FileChange, FileStatus},
        workspace::FilePanel,
    };
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let project = PathBuf::from("project");
    app.workspace
        .create_space("Work".into(), vec![project.clone()]);
    app.workspace.preferences.show_git = false;
    app.file_panel = FilePanel::Changes;
    app.changes.loaded = true;
    let roots = if grouped {
        vec![project.join("repo"), project.join("repo-other")]
    } else {
        vec![project]
    };
    app.changes.status.roots = roots.clone();
    for (index, root) in roots.iter().enumerate() {
        for path in [
            format!("src/top{index}.rs"),
            format!("src/nested/middle{index}.rs"),
            format!("src/nested/deep/leaf{index}.rs"),
            format!("src-other/sibling{index}.rs"),
        ] {
            app.changes.status.files.insert(
                root.join(path),
                FileChange {
                    status: FileStatus::Modified,
                    original: None,
                },
            );
        }
    }
    (app, roots)
}

fn assert_visible_files(app: &App, visible: &[&str], hidden: &[&str]) {
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1440., 920.), app.view());
    for file in visible {
        assert!(ui.find(*file).unwrap().visible_bounds().is_some(), "{file}");
    }
    for file in hidden {
        assert!(ui.find(*file).is_err(), "{file}");
    }
}

#[test]
fn collapsing_a_worktree_folder_keeps_all_descendants_closed_after_reopening() {
    let (mut app, _) = folder_test_app(false);
    assert_visible_files(
        &app,
        &["top0.rs", "middle0.rs", "leaf0.rs", "sibling0.rs"],
        &[],
    );
    for _ in 0..2 {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1440., 920.), app.view());
        ui.click("src").unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
    }
    assert_visible_files(
        &app,
        &["top0.rs", "sibling0.rs"],
        &["middle0.rs", "leaf0.rs"],
    );
    let _ = app.update(Message::ToggleWorktreeFolder {
        repository: None,
        path: "src/nested".into(),
    });
    assert_visible_files(
        &app,
        &["top0.rs", "middle0.rs", "sibling0.rs"],
        &["leaf0.rs"],
    );
}

#[test]
fn collapsing_a_repository_closes_its_folders_without_changing_other_repositories() {
    let (mut app, roots) = folder_test_app(true);
    for _ in 0..2 {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1440., 920.), app.view());
        ui.click(iced::widget::Id::from(format!(
            "changes-repository-{}",
            roots[0].display()
        )))
        .unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
    }
    assert_visible_files(
        &app,
        &["top1.rs", "middle1.rs", "leaf1.rs", "sibling1.rs"],
        &["top0.rs", "middle0.rs", "leaf0.rs", "sibling0.rs"],
    );
    let _ = app.update(Message::ToggleWorktreeFolder {
        repository: Some(roots[0].clone()),
        path: "src".into(),
    });
    assert_visible_files(
        &app,
        &["top0.rs", "top1.rs", "middle1.rs", "leaf1.rs"],
        &["middle0.rs", "leaf0.rs"],
    );
}

#[test]
fn grouped_folder_collapse_preserves_similar_siblings_and_other_repository_paths() {
    let (mut app, roots) = folder_test_app(true);
    for _ in 0..2 {
        let _ = app.update(Message::ToggleWorktreeFolder {
            repository: Some(roots[0].clone()),
            path: "src".into(),
        });
    }
    assert_visible_files(
        &app,
        &[
            "top0.rs",
            "sibling0.rs",
            "top1.rs",
            "middle1.rs",
            "leaf1.rs",
        ],
        &["middle0.rs", "leaf0.rs"],
    );
}

#[test]
fn uncommitted_files_are_visible_in_the_side_panel_and_open_from_the_tree() {
    use crate::domain::{
        changes::{FileChange, FileStatus},
        workspace::FilePanel,
    };
    let mut app = App::new(Rc::new(I18n::new(Language::Russian)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec![PathBuf::from("/project")]);
    app.file_panel = FilePanel::Changes;
    app.changes.loaded = true;
    let path = PathBuf::from("/project/services/changed.py");
    app.changes.status.files.insert(
        path.clone(),
        FileChange {
            status: FileStatus::Modified,
            original: None,
        },
    );
    for index in 0..50 {
        app.changes.status.files.insert(
            PathBuf::from(format!("/project/services/file{index:02}.py")),
            FileChange {
                status: FileStatus::Added,
                original: None,
            },
        );
    }
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    let bounds = ui.find("changed.py").unwrap().visible_bounds().unwrap();
    assert!(bounds.width > 0. && bounds.height > 0.);
    assert!(ui.find("M").is_err());
    let panel = ui
        .find(iced::widget::Id::new("project-files-panel"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    let browser = ui
        .find(iced::widget::Id::new("project-files-browser"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    assert_eq!(browser.x, panel.x);
    assert_eq!(browser.width, panel.width);
    assert!(ui.find("file49.py").unwrap().visible_bounds().is_none());
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&iced::Theme::Dark)
            .unwrap()
            .matches_image(output.join("uncommitted-panel"))
            .unwrap();
    }
    let top = (browser.x + browser.width - 5., browser.y + 12.);
    let bottom = (top.0, browser.y + browser.height - 14.);
    ui.point_at(top);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    ui.point_at(bottom);
    ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: bottom.into(),
        }),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ]);
    assert!(ui.find("file49.py").unwrap().visible_bounds().is_some());
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    ui.point_at(top);
    ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: top.into(),
        }),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ]);
    ui.click("changed.py").unwrap();
    assert!(
        matches!(ui.into_messages().collect::<Vec<_>>().as_slice(), [Message::OpenWorkingChange(file)] if *file == path)
    );
}

#[test]
fn status_results_cannot_cross_project_or_request_boundaries() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.generation = 4;
    app.changes.request = 2;
    app.changes.pending = true;
    let mut status = Worktree::default();
    status.folders.insert(
        PathBuf::from("/old"),
        crate::domain::changes::FileStatus::Modified,
    );
    let _ = app.update_changes(Event::Loaded(3, 2, Ok(status.clone())));
    let _ = app.update_changes(Event::Loaded(4, 1, Ok(status)));
    assert!(app.changes.status.folders.is_empty());
    assert!(app.changes.pending);
    let _ = app.update_changes(Event::Loaded(4, 2, Ok(Worktree::default())));
    assert!(!app.changes.pending);
    assert!(app.changes.loaded);
    let _ = app.update_changes(Event::Loaded(4, 2, Err(Failure::Read)));
    assert_eq!(app.changes.error, Some(Failure::Read));
    let _ = app.update_changes(Event::Loaded(4, 2, Ok(Worktree::default())));
    assert!(app.changes.error.is_none());
}

#[test]
fn background_previews_preserve_scroll_and_reject_old_file_results() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.generation = 2;
    app.file_request = 5;
    app.preview_mut().id = 5;
    app.preview_mut().scroll_y = 240.;
    app.preview_mut().document = Some(Document::new("current".to_owned(), "txt").into());
    app.changes.preview_pending = Some(4);
    let _ = app.update_changes(Event::PreviewLoaded(
        2,
        4,
        Ok(Some(Box::new(Document::new("stale".to_owned(), "txt")))),
    ));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "current");
    assert!(app.changes.preview_pending.is_none());
    let _ = app.update_changes(Event::PreviewLoaded(
        2,
        5,
        Ok(Some(Box::new(Document::new("updated".to_owned(), "txt")))),
    ));
    assert_eq!(app.preview().document.as_ref().unwrap().text, "updated");
    assert_eq!(app.preview().scroll_y, 240.);
}
