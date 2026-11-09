use super::*;
use crate::{
    app::Navigation,
    domain::workspace::FileEntry,
    i18n::{I18n, Key, Language},
};
use std::rc::Rc;

fn app() -> App {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.navigation = Navigation::Files;
    app.workspace
        .create_space("Work".to_owned(), vec![PathBuf::from("/project")]);
    let _ = app.load_file_tree();
    app
}

fn directory(app: &mut App, path: &str, entries: Vec<FileEntry>) {
    let _ = app.update_files(Event::Index(
        app.files.epoch,
        project_index::Event::Directory(path.into(), Ok(entries)),
    ));
}

fn entry(path: &str, directory: bool) -> FileEntry {
    let path = PathBuf::from(path);
    FileEntry {
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        path,
        directory,
    }
}

#[test]
fn project_contents_stay_visible_without_a_collapsible_root() {
    use iced::widget::Id;
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        directory(
            &mut app,
            "/project",
            vec![
                entry("/project/src", true),
                entry("/project/README.md", false),
            ],
        );
        directory(
            &mut app,
            "/project/src",
            vec![entry("/project/src/main.rs", false)],
        );
        app.expanded.remove(&PathBuf::from("/project"));
        let settings = || iced::Settings {
            default_font: crate::ui::theme::font(),
            ..Default::default()
        };
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        assert!(ui.find(Id::new("file-tree-/project")).is_err());
        assert!(ui.find(Id::new("space-project-0-0")).is_ok());
        assert!(ui.find("Work • project").is_ok());
        let source = ui.find("src").unwrap().visible_bounds().unwrap();
        let readme = ui.find("README.md").unwrap().visible_bounds().unwrap();
        assert_eq!(source.x, readme.x);
        assert!(ui.find("main.rs").is_err());
        ui.click(Id::new("file-tree-/project/src")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let child = ui.find("main.rs").unwrap().visible_bounds().unwrap();
        assert_eq!(child.x - source.x, 20.);
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("project-contents-{suffix}")))
                .unwrap();
        }
        ui.click(Id::new("file-tree-/project/src/main.rs")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(
            app.preview().path,
            Some(PathBuf::from("/project/src/main.rs"))
        );
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("file-tree-/project/src")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        assert!(ui.find(Id::new("file-tree-/project/src/main.rs")).is_err());
        assert!(ui.find("README.md").is_ok());
        assert_eq!(
            app.preview().path,
            Some(PathBuf::from("/project/src/main.rs"))
        );
    }
}

#[test]
fn cached_folders_stay_visible_across_refresh_and_project_switches() {
    let mut app = app();
    directory(&mut app, "/project", vec![entry("/project/src", true)]);
    directory(
        &mut app,
        "/project/src",
        vec![entry("/project/src/main.rs", false)],
    );
    let _ = app.toggle_folder("/project/src".into());
    assert!(!app.files.loading.contains(&PathBuf::from("/project/src")));
    let epoch = app.files.epoch;
    let _ = app.load_file_tree();
    assert_eq!(app.files.epoch, epoch);
    assert_eq!(app.tree[&PathBuf::from("/project/src")][0].name, "main.rs");
    app.workspace.add_project(0, "/other".into());
    let _ = app.load_file_tree();
    assert!(app.tree.is_empty());
    let _ = app.update_files(Event::Index(
        epoch,
        project_index::Event::Directory("/project".into(), Ok(vec![])),
    ));
    assert!(app.tree.is_empty());
    app.workspace.select_project(0, 0);
    let _ = app.load_file_tree();
    assert!(app.expanded.contains(&PathBuf::from("/project/src")));
    assert_eq!(app.tree[&PathBuf::from("/project/src")][0].name, "main.rs");
}

#[test]
fn loading_appears_only_for_slow_uncached_reads_and_errors_preserve_cache() {
    let mut app = app();
    let path = PathBuf::from("/project");
    let request = app.files.pending[&path];
    assert!(app.files.loading.is_empty());
    let _ = app.update_files(Event::Slow(app.files.epoch, path.clone(), request));
    assert!(app.files.loading.contains(&path));
    let cached = project_index::Tree::from([(path.clone(), vec![entry("/project/src", true)])]);
    let _ = app.update_files(Event::Index(
        app.files.epoch,
        project_index::Event::Cached(cached),
    ));
    assert!(app.files.loading.is_empty());
    let _ = app.update_files(Event::Slow(app.files.epoch, path.clone(), request));
    assert!(app.files.loading.is_empty());
    let _ = app.update_files(Event::Index(
        app.files.epoch,
        project_index::Event::Directory(path.clone(), Err(Failure::Read)),
    ));
    assert_eq!(app.files.errors[&path], Failure::Read);
    assert_eq!(app.tree[&path][0].name, "src");
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    ui.click(app.i18n.text(Key::Retry)).unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>().as_slice(),
        [Message::Files(Event::Retry(directory))] if directory == &path
    ));
    directory(&mut app, "/project", vec![]);
    assert!(app.files.errors.is_empty());
    assert!(app.tree[&path].is_empty());
}

#[test]
fn removed_directories_drop_descendants_and_cancel_loading_indicators() {
    let mut app = app();
    directory(&mut app, "/project", vec![entry("/project/src", true)]);
    let _ = app.toggle_folder("/project/src".into());
    let pending = app.files.pending[&PathBuf::from("/project/src")];
    directory(&mut app, "/project", vec![]);
    let _ = app.update_files(Event::Slow(app.files.epoch, "/project/src".into(), pending));
    assert!(!app.expanded.contains(&PathBuf::from("/project/src")));
    assert!(app.files.loading.is_empty());
}

#[test]
fn oldest_cached_project_is_restored_before_cache_eviction() {
    let mut app = app();
    directory(
        &mut app,
        "/project",
        vec![entry("/project/remember.rs", false)],
    );
    for index in 1..=4 {
        app.workspace
            .add_project(0, format!("/project-{index}").into());
        let _ = app.load_file_tree();
    }
    assert_eq!(app.files.cache.len(), 4);
    app.workspace.select_project(0, 0);
    let _ = app.load_file_tree();
    assert_eq!(app.tree[&PathBuf::from("/project")][0].name, "remember.rs");
    assert_eq!(app.files.cache.len(), 4);
}

#[test]
fn cached_tree_has_no_loading_label_in_any_language() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = app();
        app.i18n = Rc::new(I18n::new(language));
        directory(&mut app, "/project", vec![entry("/project/src", true)]);
        directory(
            &mut app,
            "/project/src",
            vec![entry("/project/src/main.rs", false)],
        );
        let _ = app.toggle_folder("/project/src".into());
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings {
                default_font: crate::ui::theme::font(),
                ..Default::default()
            },
            (1080., 720.),
            app.view(),
        );
        assert!(ui.find("main.rs").is_ok());
        assert!(ui.find(app.i18n.text(Key::Loading)).is_err());
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            let suffix = match language {
                Language::English => "en",
                Language::Russian => "ru",
                Language::Spanish => "es",
            };
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("cached-tree-{suffix}")))
                .unwrap();
        }
    }
}

#[test]
fn file_tree_centers_names_inside_each_row() {
    let mut app = app();
    directory(
        &mut app,
        "/project",
        vec![
            entry("/project/public", true),
            entry("/project/reports", true),
            entry("/project/main.rs", false),
        ],
    );
    app.files.selected_path = Some(PathBuf::from("/project/reports"));
    app.changes.status = crate::domain::changes::Worktree::new(
        std::path::Path::new("/project"),
        std::collections::BTreeMap::from([(
            PathBuf::from("/project/reports/index.rs"),
            crate::domain::changes::FileChange {
                status: crate::domain::changes::FileStatus::Modified,
                original: None,
            },
        )]),
    );
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings {
            default_font: crate::ui::theme::font(),
            ..Default::default()
        },
        (1080., 720.),
        app.view(),
    );
    for name in ["public", "reports", "main.rs"] {
        let label = ui.find(name).unwrap().visible_bounds().unwrap();
        let row = ui
            .find(iced::widget::Id::from(format!("file-tree-/project/{name}")))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!((row.height - 24.).abs() < 0.001);
        assert!((label.center_y() - row.center_y()).abs() < 0.1);
    }
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&iced::Theme::Dark)
            .unwrap()
            .matches_image(output.join("tree-centered"))
            .unwrap();
    }
}

#[test]
fn compact_tree_keeps_full_filenames_and_folder_selection() {
    let mut app = app();
    let long_name = "a_very_long_component_filename_that_must_remain_complete.tsx";
    directory(&mut app, "/project", vec![entry("/project/src", true)]);
    directory(
        &mut app,
        "/project/src",
        vec![
            entry(&format!("/project/src/{long_name}"), false),
            entry("/project/src/next.py", false),
        ],
    );
    let _ = app.toggle_folder("/project/src".into());
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    let first = ui.find(long_name).unwrap().visible_bounds().unwrap();
    let second = ui.find("next.py").unwrap().visible_bounds().unwrap();
    assert!((second.y - first.y - 24.).abs() < 0.1);
    ui.click("next.py").unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert_eq!(
        app.files.selected_path,
        Some(PathBuf::from("/project/src/next.py"))
    );
    let _ = app.toggle_folder("/project/src".into());
    assert_eq!(app.files.selected_path, Some(PathBuf::from("/project/src")));
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), (1080., 720.), app.view());
    assert!(ui.find(long_name).is_err());
}
