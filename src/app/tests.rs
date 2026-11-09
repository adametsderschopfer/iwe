use super::*;

fn ui_settings() -> iced::Settings {
    iced::Settings {
        default_font: theme::font(),
        ..Default::default()
    }
}

#[test]
fn appearance_switch_applies_immediately_in_every_language() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.navigation = Navigation::Settings;
        for (appearance, key) in [
            (Appearance::Light, Key::Light),
            (Appearance::Dark, Key::Dark),
        ] {
            let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
            ui.click(app.i18n.text(key)).unwrap();
            let messages = ui.into_messages().collect::<Vec<_>>();
            assert!(
                matches!(messages.as_slice(), [Message::SetAppearance(value)] if *value == appearance)
            );
            for message in messages {
                let _ = app.update(message);
            }
            assert_eq!(app.workspace.preferences.appearance, appearance);
            assert_eq!(
                app.theme().extended_palette().is_dark,
                appearance == Appearance::Dark
            );
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                let mut ui =
                    iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
                ui.snapshot(&app.theme())
                    .unwrap()
                    .matches_image(
                        output.join(format!("appearance-settings-{appearance:?}-{suffix}")),
                    )
                    .unwrap();
            }
        }
    }
}

#[test]
fn settings_tabs_switch_sections_without_changing_preferences() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.navigation = Navigation::Settings;
        for (tab, id, key) in [
            (SettingsTab::General, "general", Key::LanguageDescription),
            (SettingsTab::Code, "code", Key::FontSizeDescription),
            (SettingsTab::Panels, "panels", Key::GitDescription),
            (
                SettingsTab::Shortcuts,
                "shortcuts",
                if cfg!(target_os = "macos") {
                    Key::CloseApplicationDescription
                } else {
                    Key::CloseActiveTabDescription
                },
            ),
            (SettingsTab::About, "about", Key::AboutDescription),
        ] {
            let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
            let bounds = ui
                .click(iced::widget::Id::from(format!("settings-tab-{id}")))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(bounds.x + bounds.width <= 1080.);
            for message in ui.into_messages() {
                let _ = app.update(message);
            }
            assert_eq!(app.settings_tab, tab);
            let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
            assert!(
                ui.find(app.i18n.text(key))
                    .unwrap()
                    .visible_bounds()
                    .is_some()
            );
            if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
                ui.snapshot(&Theme::Dark)
                    .unwrap()
                    .matches_image(output.join(format!("settings-{id}-{suffix}")))
                    .unwrap();
            }
            assert_eq!(app.workspace.preferences.code_size, 14);
        }
    }
}

#[test]
fn branch_folders_remotes_and_tags_start_collapsed() {
    use crate::domain::repository::{Branch, ReferenceKind};
    let mut app = history_state(3, false);
    app.repository.as_mut().unwrap().branches = [
        (
            "feature/current",
            "refs/heads/feature/current",
            ReferenceKind::Local,
            true,
        ),
        (
            "backup/saved",
            "refs/heads/backup/saved",
            ReferenceKind::Local,
            false,
        ),
        (
            "origin/main",
            "refs/remotes/origin/main",
            ReferenceKind::Remote,
            false,
        ),
        ("v1.0", "refs/tags/v1.0", ReferenceKind::Tag, false),
    ]
    .into_iter()
    .map(|(name, reference, kind, current)| Branch {
        name: name.into(),
        reference: reference.into(),
        kind,
        current,
    })
    .collect();
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
    assert!(
        ui.find(iced::widget::Id::new("branch-refs/heads/feature/current"))
            .is_ok()
    );
    assert!(ui.find("backup").is_ok());
    for hidden in ["saved", "origin", "v1.0"] {
        assert!(ui.find(hidden).is_err());
    }
    ui.click("backup").unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
    assert!(ui.find("saved").is_ok());
    drop(ui);
    let _ = app.load_directory();
    assert_eq!(
        app.expanded_branch_folders,
        BTreeSet::from(["false".to_owned()])
    );
}

#[test]
fn commit_dates_with_time_fit_one_line_and_remain_visible() {
    for width in [1080., 1440.] {
        let mut app = history_state(1, false);
        app.i18n = Rc::new(I18n::new(Language::Russian));
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (width, 720.), app.view());
        let date = ui
            .find("2026-11-08 14:37")
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(date.width >= theme::COMMIT_DATE_WIDTH - 0.1);
        assert!(date.height <= 11. * 1.3 + 0.1);
        assert!(date.x + date.width <= width);
        if width == 1080.
            && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
        {
            ui.snapshot(&Theme::Dark)
                .unwrap()
                .matches_image(output.join("commit-date-time"))
                .unwrap();
        }
        drop(ui);
        let id = app.repository.as_ref().unwrap().commits[0].id.clone();
        let _ = app.update(Message::SelectCommit(id.clone()));
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (width, 720.), app.view());
        assert!(
            ui.find(format!("{} · 2026-11-08 14:37", &id[..8]))
                .unwrap()
                .visible_bounds()
                .is_some()
        );
    }
}

#[test]
fn closing_commit_files_clears_selection_and_rejects_pending_changes() {
    let mut app = history_state(3, false);
    let capture = |app: &App, name: &str| {
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            iced_test::Simulator::with_size(ui_settings(), (1440., 920.), app.view())
                .snapshot(&Theme::Dark)
                .unwrap()
                .matches_image(output.join(name))
                .unwrap();
        }
    };
    assert!(app.selected_commit.is_none());
    assert_eq!(app.git_panes.len(), 2);
    capture(&app, "commit-unselected");
    let id = app.repository.as_ref().unwrap().commits[0].id.clone();
    let _ = app.update(Message::SelectCommit(id.clone()));
    assert_eq!(app.git_panes.len(), 3);
    let _ = app.update(Message::ChangesLoaded(
        app.generation,
        app.commit_request,
        Ok(vec![ChangedFile {
            path: "src/main.rs".into(),
            status: 'M',
        }]),
    ));
    capture(&app, "commit-selected");
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1440., 920.), app.view());
    let file = ui.find("main.rs").unwrap().visible_bounds().unwrap();
    assert!(file.width > 0. && file.height > 0.);
    ui.click("main.rs").unwrap();
    let messages = ui.into_messages().collect::<Vec<_>>();
    assert!(matches!(messages.as_slice(), [Message::OpenCommitFile(0)]));
    for message in messages {
        let _ = app.update(message);
    }
    let preview = app.preview().id;
    let _ = app.update(Message::SelectCommit(id.clone()));
    let request = app.commit_request;
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1440., 920.), app.view());
    ui.click(iced::widget::Id::new("close-commit-files"))
        .unwrap();
    for message in ui.into_messages() {
        let _ = app.update(message);
    }
    assert!(app.selected_commit.is_none());
    assert_eq!(app.git_panes.len(), 2);
    assert_eq!(app.preview().id, preview);
    assert_eq!(app.preview().revision.as_ref(), Some(&id));
    let _ = app.update(Message::FileLoaded(
        app.generation,
        preview,
        Ok(Document::new("commit source".into(), "rs")),
    ));
    assert_eq!(
        app.preview().document.as_ref().unwrap().text,
        "commit source"
    );
    let _ = app.update(Message::ChangesLoaded(
        app.generation,
        request,
        Ok(vec![ChangedFile {
            path: "late.rs".into(),
            status: 'M',
        }]),
    ));
    assert!(app.changed_files.is_empty());
    assert!(!app.changes_loading);
    let _ = app.update(Message::SelectCommit(id));
    assert_eq!(app.git_panes.len(), 3);
    let _ = app.update(Message::SelectBranch("refs/heads/main".into()));
    assert_eq!(app.git_panes.len(), 2);
    assert!(app.selected_commit.is_none());
}

#[test]
fn pipeline_badges_match_full_commits_and_refresh_without_flicker() {
    use crate::domain::repository::Remote;
    let mut app = history_state(3, false);
    app.repository.as_mut().unwrap().remote = Remote::parse("https://github.com/team/app.git");
    let commit = app.repository.as_ref().unwrap().commits[0].id.clone();
    let run = |id, sha: String, status: &str| Pipeline {
        id,
        number: None,
        title: "Build".into(),
        branch: "main".into(),
        commit: Some(sha),
        status: status.into(),
        date: String::new(),
        url: format!("https://github.com/team/app/actions/runs/{id}"),
    };
    app.pipelines = PipelineState::Ready(vec![
        run(1, commit.clone(), "in_progress"),
        run(2, commit[..7].to_owned(), "running"),
        run(3, commit.clone(), "success"),
    ]);
    assert_eq!(app.active_pipelines(&commit).len(), 1);
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
    assert!(
        ui.find(iced::widget::Id::from(format!("commit-pipeline-{commit}")))
            .is_ok()
    );
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&Theme::Dark)
            .unwrap()
            .matches_image(output.join("pipeline-in-history"))
            .unwrap();
    }
    drop(ui);
    let _ = app.update(Message::RefreshPipelines);
    let request = app.pipeline_request;
    assert_eq!(app.active_pipelines(&commit).len(), 1);
    let _ = app.update(Message::RefreshPipelines);
    assert_eq!(app.pipeline_request, request);
    let _ = app.update(Message::PipelinesLoaded(
        app.generation,
        request,
        pipelines::Loaded {
            runs: Ok(vec![run(1, commit.clone(), "success")]),
            username: None,
        },
    ));
    assert!(app.active_pipelines(&commit).is_empty());
    let _ = app.update(Message::PipelinesLoaded(
        app.generation,
        request - 1,
        pipelines::Loaded {
            runs: Ok(vec![run(1, commit.clone(), "running")]),
            username: None,
        },
    ));
    assert!(app.active_pipelines(&commit).is_empty());
    let _ = app.update(Message::RefreshPipelines);
    let _ = app.update(Message::PipelinesLoaded(
        app.generation,
        app.pipeline_request,
        pipelines::Loaded {
            runs: Err(Failure::Unavailable),
            username: None,
        },
    ));
    assert!(!app.pipeline_refreshing);
    assert!(app.active_pipelines(&commit).is_empty());
}

#[test]
fn project_refresh_retains_git_and_pipeline_availability_until_results_arrive() {
    use crate::domain::repository::Remote;
    let mut app = history_state(3, false);
    // A project can point inside the repository instead of its Git root.
    app.workspace.spaces[0].projects[0].directory = PathBuf::from("project/nested");
    app.project_directory = app.workspace.directory().cloned();
    app.repository.as_mut().unwrap().remote = Remote::parse("https://github.com/team/app.git");
    let repo = app.repository.as_ref().unwrap().clone();
    app.pipelines = PipelineState::Ready(Vec::new());
    app.account_username = Some("account".into());
    app.workspace.preferences.show_git = false;
    app.workspace.preferences.show_pipelines = true;
    app.sync_panels();
    let pipeline_pane = *app
        .panes
        .iter()
        .find(|(_, panel)| **panel == Panel::Pipelines)
        .unwrap()
        .0;
    let previous_generation = app.generation;
    let _ = app.update(Message::Refresh);
    assert!(app.loading);
    assert!(app.has_remote());
    assert_eq!(app.repository.as_ref().unwrap().commits.len(), 3);
    assert!(matches!(app.pipelines, PipelineState::Ready(_)));
    assert_eq!(app.account_username.as_deref(), Some("account"));
    assert_eq!(app.panes.get(pipeline_pane), Some(&Panel::Pipelines));
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
    assert!(
        ui.find(iced::widget::Id::new("nav-pipelines"))
            .unwrap()
            .visible_bounds()
            .is_some()
    );
    assert!(ui.find("account").unwrap().visible_bounds().is_some());
    drop(ui);
    let _ = app.update(Message::RepositoryLoaded(
        previous_generation,
        app.repo_request,
        Err(Failure::NotRepository),
    ));
    assert!(app.has_remote());
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Ok(repo.clone()),
    ));
    assert!(app.pipeline_refreshing);
    assert!(matches!(app.pipelines, PipelineState::Ready(_)));
    let _ = app.update(Message::PipelinesLoaded(
        app.generation,
        app.pipeline_request,
        pipelines::Loaded {
            runs: Ok(Vec::new()),
            username: Some("account".into()),
        },
    ));
    assert!(!app.pipeline_refreshing);

    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Err(Failure::Read),
    ));
    assert!(app.has_remote());
    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Ok(Repository {
            remotes: vec!["origin".to_owned()],
            remote: None,
            ..repo
        }),
    ));
    assert!(!app.has_remote());
    assert!(app.account_username.is_none());
    assert!(
        !app.panes
            .iter()
            .any(|(_, panel)| *panel == Panel::Pipelines)
    );
}

#[test]
fn refresh_discards_confirmed_removed_repositories_and_does_not_reuse_other_projects() {
    use crate::domain::repository::Remote;
    let mut app = history_state(1, false);
    app.repository.as_mut().unwrap().remote = Remote::parse("https://github.com/team/app.git");
    app.pipelines = PipelineState::Ready(Vec::new());
    let repo = app.repository.as_ref().unwrap().clone();
    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Err(Failure::NotRepository),
    ));
    assert!(app.repository.is_none());
    assert!(!app.has_remote());
    assert!(matches!(app.pipelines, PipelineState::Idle));
    let _ = app.update(Message::Refresh);
    let _ = app.update(Message::RepositoryLoaded(
        app.generation,
        app.repo_request,
        Ok(repo),
    ));
    assert!(app.has_remote());
    app.workspace
        .create_space("Other".into(), vec![PathBuf::from("other")]);
    let _ = app.load_directory();
    assert!(app.repository.is_none());
    assert!(!app.has_remote());
    assert!(matches!(app.pipelines, PipelineState::Idle));
}

#[test]
fn window_and_tab_shortcuts_support_keyboard_layouts_without_extra_shortcuts() {
    use iced::{
        Event,
        keyboard::{Event as KeyEvent, Key, Modifiers, key},
    };
    let event = |name: &str, code, modifiers, repeat| {
        Event::Keyboard(KeyEvent::KeyPressed {
            key: Key::Character(name.into()),
            modified_key: Key::Character(name.into()),
            physical_key: key::Physical::Code(code),
            location: keyboard::Location::Standard,
            modifiers,
            repeat,
            text: None,
        })
    };
    for name in ["w", "W", "ц"] {
        let message = shortcut(event(name, key::Code::KeyW, Modifiers::COMMAND, false));
        if cfg!(target_os = "macos") {
            assert!(matches!(message, Some(Message::CloseApplication)));
        } else {
            assert!(matches!(message, Some(Message::CloseActiveTab)));
        }
    }
    for name in ["t", "T", "е"] {
        assert!(matches!(
            shortcut(event(
                name,
                key::Code::KeyT,
                Modifiers::COMMAND | Modifiers::SHIFT,
                false
            )),
            Some(Message::ReopenClosedTab)
        ));
    }
    for modifiers in [
        Modifiers::NONE,
        Modifiers::COMMAND,
        Modifiers::SHIFT,
        Modifiers::COMMAND | Modifiers::SHIFT | Modifiers::ALT,
    ] {
        assert!(shortcut(event("t", key::Code::KeyT, modifiers, false)).is_none());
    }
    assert!(
        shortcut(event(
            "T",
            key::Code::KeyT,
            Modifiers::COMMAND | Modifiers::SHIFT,
            true
        ))
        .is_none()
    );
    for modifiers in [
        Modifiers::NONE,
        Modifiers::COMMAND | Modifiers::SHIFT,
        Modifiers::COMMAND | Modifiers::ALT,
    ] {
        assert!(shortcut(event("w", key::Code::KeyW, modifiers, false)).is_none());
    }
    assert!(shortcut(event("w", key::Code::KeyW, Modifiers::COMMAND, true)).is_none());
    for (name, code) in [
        ("1", key::Code::Digit1),
        ("2", key::Code::Digit2),
        (",", key::Code::Comma),
    ] {
        assert!(shortcut(event(name, code, Modifiers::COMMAND, false)).is_none());
    }
    for key in [key::Named::F5, key::Named::Tab] {
        assert!(shortcut(iced_test::simulator::press_key(key, None)).is_none());
    }
}

#[test]
fn selecting_projects_immediately_opens_the_file_panel_without_reloading_on_repeat() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.loaded = true;
        app.workspace
            .create_space("Team".to_owned(), vec![PathBuf::from("frontend")]);
        app.workspace
            .create_space("Personal".to_owned(), vec![PathBuf::from("notes")]);
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        ui.click(Id::new("space-project-0-0")).unwrap();
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(matches!(
            messages.as_slice(),
            [Message::SelectProject(0, 0)]
        ));
        let _ = app.update(messages.into_iter().next().unwrap());
        assert_eq!(app.workspace.directory(), Some(&PathBuf::from("frontend")));
        let generation = app.generation;
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        let panel = ui
            .find(Id::new("project-files-panel"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(panel.x, theme::SIDEBAR_WIDTH + 1. + 48.);
        assert_eq!(panel.width, app.workspace.preferences.file_panel_width);
        assert!(ui.find(Id::new("space-project-0-0")).is_ok());
        ui.click(Id::new("space-project-0-0")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.generation, generation);
        let _ = app.update(Message::SelectFilePanel(FilePanel::Changes));
        app.changes.loaded = true;
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        let title = ui
            .find(Id::new("file-panel-title"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let empty = ui
            .find(app.i18n.text(Key::NoWorkingChanges))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let first = ui
            .find(Id::new("file-panel-project"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let second = ui
            .find(Id::new("file-panel-changes"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert_eq!(title.x, empty.x);
        assert!(empty.y > title.y + title.height);
        assert!((title.center_y() - first.center_y()).abs() < 0.1);
        assert_eq!(first.center_x(), second.center_x());
        assert_eq!(first.center_x(), (theme::SIDEBAR_WIDTH + 1. + panel.x) / 2.);
        if language == Language::Russian
            && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
        {
            for (theme, suffix) in [(Theme::Dark, "dark"), (Theme::Light, "light")] {
                ui.snapshot(&theme)
                    .unwrap()
                    .matches_image(output.join(format!("file-panel-spacing-{suffix}")))
                    .unwrap();
            }
        }
        ui.click(Id::new("file-panel-project")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.file_panel, FilePanel::Project);
        assert_eq!(app.generation, generation);
        let _ = app.update(Message::SelectProject(1, 0));
        assert_eq!(app.workspace.directory(), Some(&PathBuf::from("notes")));
        assert!(app.generation > generation);
    }
}

#[test]
fn shortcuts_are_visible_in_settings_in_every_language() {
    use iced::{Event, mouse};
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        app.navigation = Navigation::Settings;
        app.settings_tab = SettingsTab::Shortcuts;
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        ui.point_at((900., 600.));
        ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y: -650. },
        })]);
        for label in [
            app.i18n.text(Key::KeyboardShortcuts),
            app.i18n.text(if cfg!(target_os = "macos") {
                Key::CloseApplicationDescription
            } else {
                Key::CloseActiveTabDescription
            }),
            app.i18n.text(Key::ReopenClosedTabDescription),
            if cfg!(target_os = "macos") {
                "Command + Shift + T"
            } else {
                "Ctrl + Shift + T"
            }
            .to_owned(),
            if cfg!(target_os = "macos") {
                "Command + W"
            } else {
                "Ctrl + W"
            }
            .to_owned(),
        ] {
            assert!(ui.find(label).unwrap().visible_bounds().is_some());
        }
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("shortcuts-{suffix}")))
                .unwrap();
        }
    }
}

fn file_panel_app(language: Language) -> App {
    let mut app = App::new(Rc::new(I18n::new(language)));
    app.loaded = true;
    app.workspace
        .create_space("Work".into(), vec![PathBuf::from("project")]);
    app.workspace.preferences.show_git = false;
    app.sync_panels();
    let _ = app.open_file(PathBuf::from("project/example.rs"));
    let _ = app.finish_preview(
        app.preview().id,
        Ok(Document::new("let value = 42;\n".repeat(200), "rs")),
    );
    app.changes.loaded = true;
    app
}

#[test]
fn file_panel_buttons_toggle_without_changing_open_files() {
    use iced::widget::Id;
    let mut app = file_panel_app(Language::Russian);
    let active = app.preview().id;
    let document = app.preview().document.as_ref().unwrap().text.as_ptr();
    app.preview_mut().scroll_y = 100.;
    app.preview_mut().scroll_x = 40.;
    let generation = app.generation;
    for (button, visible, panel) in [
        ("file-panel-project", false, FilePanel::Project),
        ("file-panel-project", true, FilePanel::Project),
        ("file-panel-changes", true, FilePanel::Changes),
        ("file-panel-changes", false, FilePanel::Changes),
        ("file-panel-project", true, FilePanel::Project),
    ] {
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        ui.click(Id::new(button)).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.file_panel_visible, visible);
        assert_eq!(app.file_panel, panel);
        assert_eq!(app.preview().id, active);
        assert_eq!(
            app.preview().document.as_ref().unwrap().text.as_ptr(),
            document
        );
        assert_eq!(
            (app.preview().scroll_x, app.preview().scroll_y),
            (40., 100.)
        );
        assert_eq!(app.generation, generation);
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
        let source = ui
            .find(Id::new("source-preview"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let expected_x = theme::SIDEBAR_WIDTH
            + 1.
            + 48.
            + if visible {
                app.workspace.preferences.file_panel_width + 1.
            } else {
                0.
            };
        assert_eq!(source.x, expected_x);
        assert_eq!(source.width, 1080. - expected_x);
        let files = ui
            .find(Id::new("project-files-panel"))
            .ok()
            .and_then(|target| target.visible_bounds());
        assert_eq!(files.is_some_and(|bounds| bounds.width > 0.), visible);
    }
    let _ = app.update(Message::ToggleFilePanel(FilePanel::Project));
    assert!(!app.file_panel_visible);
    let _ = app.update(Message::SelectFilePanel(FilePanel::Project));
    assert!(app.file_panel_visible);
    let _ = app.update(Message::SelectFilePanel(FilePanel::Project));
    assert!(app.file_panel_visible);
}

#[test]
fn file_panel_resize_saves_on_release_and_preserves_source_room_and_header_alignment() {
    use crate::domain::workspace::Preferences;
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = file_panel_app(language);
        let id = app.preview().id;
        assert!(!app.saving);
        let _ = app.update(Message::ResizeFilePanel(400., false));
        assert_eq!(app.workspace.preferences.file_panel_width, 400.);
        assert!(!app.saving);
        let _ = app.update(Message::ResizeFilePanel(400., true));
        assert!(app.saving);
        assert_eq!(app.preview().id, id);
        let _ = app.update(Message::Saved(Ok(())));
        let _ = app.update(Message::ResizeFilePanel(f32::NAN, true));
        assert_eq!(app.workspace.preferences.file_panel_width, 400.);
        assert!(!app.saving);
        for requested in [
            Preferences::MIN_FILE_PANEL_WIDTH,
            400.,
            Preferences::MAX_FILE_PANEL_WIDTH,
        ] {
            let _ = app.update(Message::ResizeFilePanel(requested, false));
            let _ = app.update(Message::SelectFilePanel(FilePanel::Changes));
            for width in [1080., 1440.] {
                let mut ui =
                    iced_test::Simulator::with_size(ui_settings(), (width, 720.), app.view());
                let files = ui
                    .find(Id::new("project-files-panel"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let source = ui
                    .find(Id::new("source-preview"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(source.width >= theme::SOURCE_MIN_WIDTH);
                assert_eq!(source.x, files.x + files.width + 1.);
                assert!(files.width <= requested);
                let heading = ui
                    .find(Id::new("file-panel-header"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let tabs = ui
                    .find(Id::new("file-tabs-header"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert_eq!(heading.y, tabs.y);
                assert_eq!(heading.y + heading.height + 1., tabs.y + tabs.height);
                let title = ui
                    .find(Id::new("file-panel-title"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let filename = ui.find("example.rs").unwrap().visible_bounds().unwrap();
                assert!((title.center_y() - filename.center_y()).abs() <= 1.);
                if language == Language::Russian
                    && width == 1440.
                    && requested == 400.
                    && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
                {
                    for (theme, suffix) in [(Theme::Dark, "dark"), (Theme::Light, "light")] {
                        ui.snapshot(&theme)
                            .unwrap()
                            .matches_image(output.join(format!("resized-file-panel-{suffix}")))
                            .unwrap();
                    }
                }
            }
            assert_eq!(app.workspace.preferences.file_panel_width, requested);
        }
        let _ = app.update(Message::ToggleFilePanel(FilePanel::Changes));
        let width = app.workspace.preferences.file_panel_width;
        let _ = app.update(Message::ResizeFilePanel(200., true));
        assert_eq!(app.workspace.preferences.file_panel_width, width);
        assert!(!app.saving);
    }
}

#[test]
fn checkout_precedes_main_master_and_alphabetical_branches() {
    use crate::domain::repository::{Branch, ReferenceKind};
    use iced::widget::Id;
    let mut app = history_state(3, false);
    let mut branches = Vec::new();
    for (kind, prefix, names) in [
        (
            ReferenceKind::Local,
            "refs/heads/",
            vec![
                "dev",
                "master",
                "main",
                "aaa",
                "feature/current",
                "feature/another",
            ],
        ),
        (
            ReferenceKind::Remote,
            "refs/remotes/",
            vec!["origin/dev", "origin/master", "origin/main", "origin/aaa"],
        ),
    ] {
        branches.extend(names.into_iter().map(|name| Branch {
            name: name.to_owned(),
            reference: format!("{prefix}{name}"),
            current: name == "feature/current",
            kind,
        }));
    }
    app.repository.as_mut().unwrap().branches = branches;
    app.branch = Some("refs/heads/master".to_owned());
    app.expanded_branch_folders
        .extend(["true".to_owned(), "true/origin".to_owned()]);
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1440., 920.), app.view());
    let mut previous_y = 0.;
    for reference in [
        "heads/feature/current",
        "heads/main",
        "heads/master",
        "heads/aaa",
        "heads/dev",
        "remotes/origin/main",
        "remotes/origin/master",
        "remotes/origin/aaa",
        "remotes/origin/dev",
    ] {
        let bounds = ui
            .find(Id::from(format!("branch-refs/{reference}")))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(
            bounds.y > previous_y,
            "{reference} must follow the previous branch"
        );
        previous_y = bounds.y;
    }
    assert!(
        ui.find(Id::new("branch-refs/heads/feature/another"))
            .is_err()
    );
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&Theme::Dark)
            .unwrap()
            .matches_image(output.join("branch-order"))
            .unwrap();
    }
}

#[test]
fn large_scrollbar_thumbs_remain_grabbable_and_reach_both_ends() {
    use iced::{
        Event, Length, mouse,
        widget::{scrollable, space},
    };
    for (start, end, vertical) in [
        ((315., 20.), (315., 230.), true),
        ((20., 235.), (310., 235.), false),
    ] {
        let simulate = |drag: bool, back: bool| {
            let content: iced::widget::Scrollable<'_, operation::AbsoluteOffset> =
                scrollable(space().width(1_000_000).height(1_000_000))
                    .direction(scrollable::Direction::Both {
                        vertical: scrollable::Scrollbar::default(),
                        horizontal: scrollable::Scrollbar::default(),
                    })
                    .on_scroll(|viewport| viewport.absolute_offset())
                    .width(Length::Fill)
                    .height(Length::Fill);
            let mut ui = iced_test::Simulator::with_size(ui_settings(), (320., 240.), content);
            ui.point_at(start);
            ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))]);
            if drag {
                ui.point_at(end);
                ui.simulate([
                    Event::Mouse(mouse::Event::CursorMoved {
                        position: end.into(),
                    }),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ]);
            }
            if back {
                let grabbed = if vertical {
                    (end.0, end.1 - 1.)
                } else {
                    (end.0 - 1., end.1)
                };
                let origin = if vertical {
                    (start.0, 0.)
                } else {
                    (0., start.1)
                };
                ui.point_at(grabbed);
                ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                ))]);
                ui.point_at(origin);
                ui.simulate([
                    Event::Mouse(mouse::Event::CursorMoved {
                        position: origin.into(),
                    }),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ]);
            }
            ui.into_messages().collect::<Vec<_>>()
        };
        let pressed = simulate(false, false);
        assert!(
            pressed
                .iter()
                .all(|offset| offset.x == 0. && offset.y == 0.),
            "Pressing 20 pixels into the thumb must grab it without jumping"
        );
        let moved = simulate(true, false);
        let last = moved.last().expect("Dragging must emit a scroll offset");
        let (offset, maximum) = if vertical {
            (last.y, 1_000_000. - 240.)
        } else {
            (last.x, 1_000_000. - 320.)
        };
        assert!(
            (offset - maximum).abs() < 1.,
            "Dragging must reach the end: {offset} / {maximum}"
        );
        let returned = simulate(true, true);
        let last = returned.last().unwrap();
        assert!(
            last.x.abs() < 1. && last.y.abs() < 1.,
            "Dragging back must reach the start"
        );
    }
}

#[tokio::test]
async fn hiding_and_resizing_the_file_panel_preserves_scroll_widget_state() {
    use iced::{
        Rectangle, Vector,
        widget::{Id, scrollable::AbsoluteOffset},
    };
    use iced_test::{
        core::{
            clipboard,
            renderer::Headless,
            widget::{Operation, operation},
        },
        runtime::{UserInterface, user_interface},
    };
    #[derive(Default)]
    struct SourceOffset(Option<Vector>);
    impl Operation for SourceOffset {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _: Rectangle,
            _: Rectangle,
            translation: Vector,
            _: &mut dyn operation::Scrollable,
        ) {
            if id == Some(&Id::new("source-code")) {
                self.0 = Some(translation);
            }
        }
    }
    let mut app = file_panel_app(Language::English);
    let _ = app.finish_preview(
        app.preview().id,
        Ok(Document::plain(
            format!("{}\n", "x".repeat(400)).repeat(200),
        )),
    );
    let mut renderer =
        iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    // Initialize the pane grid before scrolling and rebuilding the view.
    let mut cache = UserInterface::build(
        app.view(),
        iced::Size::new(1080., 720.),
        user_interface::Cache::default(),
        &mut renderer,
    )
    .into_cache();
    for (index, action) in [
        None,
        None,
        Some(Message::ToggleFilePanel(FilePanel::Project)),
        Some(Message::ToggleFilePanel(FilePanel::Project)),
        Some(Message::ResizeFilePanel(400., false)),
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(action) = action {
            let _ = app.update(action);
        }
        let mut ui = UserInterface::build(
            app.view(),
            iced::Size::new(1080., 720.),
            cache,
            &mut renderer,
        );
        let mut messages = Vec::new();
        ui.update(
            &[iced::Event::Window(window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ))],
            iced::mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        if index == 0 {
            ui.operate(
                &renderer,
                &mut operation::scrollable::scroll_to(
                    Id::new("source-code"),
                    AbsoluteOffset {
                        x: Some(90.),
                        y: Some(300.),
                    },
                ),
            );
            ui.update(
                &[iced::Event::Window(window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                iced::mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
        }
        let mut offset = SourceOffset::default();
        ui.operate(&renderer, &mut offset);
        assert_eq!(
            offset.0,
            Some(Vector::new(90., 300.)),
            "Layout step {index}"
        );
        cache = ui.into_cache();
        for message in messages {
            let _ = app.update(message);
        }
    }
}

#[tokio::test]
async fn resizing_reuses_layout_without_scaling_text_or_adding_source_insets() {
    use iced_test::{
        Selector,
        core::{
            clipboard,
            renderer::Headless,
            widget::{Operation, operation},
        },
        runtime::{UserInterface, user_interface},
    };
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.loaded = true;
    app.navigation = Navigation::Files;
    app.workspace
        .create_space("Work".into(), vec![PathBuf::from("project")]);
    let _ = app.open_file(PathBuf::from("project/resize.rs"));
    let _ = app.finish_preview(
        app.preview().id,
        Ok(Document::new("let value = 42;\n".repeat(80), "rs")),
    );
    let mut renderer =
        iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let mut ui = UserInterface::build(
        app.view(),
        iced::Size::new(1080., 720.),
        user_interface::Cache::default(),
        &mut renderer,
    );
    let mut original_line_size = None;
    for (width, height) in [
        (1080., 720.),
        (1100., 730.),
        (1440., 920.),
        (1800., 1100.),
        (1300., 800.),
        (1080., 720.),
    ] {
        ui = ui.relayout(iced::Size::new(width, height), &mut renderer);
        let mut messages = Vec::new();
        ui.update(
            &[iced::Event::Window(window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ))],
            iced::mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        let mut source = Selector::find(iced::widget::Id::new("source-code"));
        let mut sidebar = Selector::find(iced::widget::Id::new("project-browser"));
        ui.operate(&renderer, &mut operation::black_box(&mut source));
        ui.operate(&renderer, &mut operation::black_box(&mut sidebar));
        let operation::Outcome::Some(Some(source)) = source.finish() else {
            panic!("Source must remain visible")
        };
        let operation::Outcome::Some(Some(sidebar)) = sidebar.finish() else {
            panic!("Sidebar must remain visible")
        };
        let source = source.visible_bounds().unwrap();
        let mut number = Selector::find(|candidate: iced_test::selector::Candidate<'_>| {
            "1".select(candidate).filter(|number| {
                number
                    .visible_bounds()
                    .is_some_and(|bounds| source.contains(bounds.center()))
            })
        });
        ui.operate(&renderer, &mut operation::black_box(&mut number));
        let operation::Outcome::Some(Some(number)) = number.finish() else {
            panic!("First line must remain visible")
        };
        let number = number.visible_bounds().unwrap();
        let sidebar = sidebar.visible_bounds().unwrap();
        assert_eq!(
            source.x,
            theme::SIDEBAR_WIDTH + 48. + app.workspace.preferences.file_panel_width + 2.
        );
        assert_eq!(
            source.width,
            width - theme::SIDEBAR_WIDTH - 48. - app.workspace.preferences.file_panel_width - 2.
        );
        assert_eq!(number.y, source.y);
        assert_eq!(number.x - source.x, 13.);
        assert_eq!(sidebar.width, theme::SIDEBAR_WIDTH - 24.);
        let line_size = (number.width, number.height);
        assert_eq!(*original_line_size.get_or_insert(line_size), line_size);
        assert!(
            messages.iter().all(|message| matches!(
                message,
                Message::CodeScrolled { id, x: 0., y: 0. } if *id == app.preview().id
            )),
            "Resizing must not move source content: {messages:?}"
        );
    }
}

#[tokio::test]
async fn code_scroll_preserves_position_and_reaches_the_last_line() {
    check_code_scrolling(2000).await;
}

#[tokio::test]
async fn short_code_scroll_preserves_position_and_reaches_the_last_line() {
    check_code_scrolling(45).await;
}

async fn check_code_scrolling(line_count: usize) {
    use iced::{Event, mouse};
    use iced_test::{
        Selector,
        core::{clipboard, renderer::Headless, widget::Operation},
        runtime::{UserInterface, user_interface},
    };

    let mut state = App::new(Rc::new(I18n::new(Language::English)));
    state.loaded = true;
    state.preview_mut().title = "large.rs".to_owned();
    state.preview_mut().document = Some(Document::new(
        (0..line_count)
            .map(|index| format!("let value_{index} = format!(\"row {{}}: {{:?}}\", {index}, Some(\"highlighted content\"));\n"))
            .collect(),
        "rs",
    ).into());
    let mut renderer =
        iced_test::renderer::Renderer::new(theme::font(), 16.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let window = iced::Size::new(1080., 720.);
    let bounds = {
        let mut ui = iced_test::Simulator::with_size(ui_settings(), window, state.view());
        ui.find(iced::widget::Id::new("source-code"))
            .unwrap()
            .visible_bounds()
            .unwrap()
    };
    let cursor = mouse::Cursor::Available(bounds.center());
    let mut cache = user_interface::Cache::default();
    // Initialize pane ordering before simulating user input.
    for _ in 0..2 {
        cache = UserInterface::build(state.view(), window, cache, &mut renderer).into_cache();
    }
    let mut timings = Vec::new();
    let deltas: Vec<f32> = if line_count == 45 {
        [24., -24.]
            .into_iter()
            .cycle()
            .take(160)
            .chain([1_000_000.])
            .collect()
    } else {
        std::iter::repeat_n(24., 120)
            .chain(std::iter::repeat_n(-12., 40))
            .chain([1_000_000.])
            .collect()
    };
    for delta in deltas {
        let started = std::time::Instant::now();
        let mut ui = UserInterface::build(state.view(), window, cache, &mut renderer);
        timings.push(started.elapsed().as_secs_f64() * 1000.);
        let previous = state.preview().scroll_y;
        let mut messages = Vec::new();
        ui.update(
            &[
                Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                )),
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0., y: -delta },
                }),
            ],
            cursor,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        cache = ui.into_cache();
        for message in messages {
            let _ = state.update(message);
        }
        if delta < 1_000_000. {
            assert!(
                (state.preview().scroll_y - previous - delta).abs() < 0.1,
                "Rebuilding visible rows must preserve the scroll offset: previous={previous}, delta={delta}, current={}",
                state.preview().scroll_y
            );
        }
    }
    let mut ui = UserInterface::build(state.view(), window, cache, &mut renderer);
    let last_label = line_count.to_string();
    let mut last_line = Selector::find(last_label.as_str());
    ui.operate(
        &renderer,
        &mut iced_test::core::widget::operation::black_box(&mut last_line),
    );
    let iced_test::core::widget::operation::Outcome::Some(Some(last)) = last_line.finish() else {
        panic!("The final line must be present after scrolling to the bottom");
    };
    let last = last.visible_bounds().unwrap();
    // Iced aligns scroll offsets to half pixels; fractional line heights can round the bottom edge.
    assert!(
        last.y >= bounds.y && last.y + last.height <= bounds.y + bounds.height - 11.5,
        "last={last:?}, viewport={bounds:?}, scroll={}",
        state.preview().scroll_y
    );
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        let mut capture = iced_test::Simulator::with_size(ui_settings(), window, state.view());
        capture.point_at(bounds.center());
        capture.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels {
                x: 0.,
                y: -1_000_000.,
            },
        })]);
        capture.point_at((0., 0.));
        capture
            .snapshot(&Theme::Dark)
            .unwrap()
            .matches_image(output.join(format!("code-end-{line_count}")))
            .unwrap();
    }
    timings.remove(0);
    timings.sort_by(f64::total_cmp);
    eprintln!(
        "Code scroll layout ({line_count} lines): median {:.2} ms, p95 {:.2} ms",
        timings[timings.len() / 2],
        timings[timings.len() * 95 / 100],
    );
}

fn history_state(count: usize, more: bool) -> App {
    use crate::domain::repository::{Commit, GraphRow, HistoryCursor, assign_graph};
    let id = |number: usize| format!("{number:040x}");
    let mut commits: Vec<_> = (1..=count)
        .rev()
        .map(|number| Commit {
            id: id(number + usize::from(more)),
            parents: (number > 1 || more)
                .then(|| id(number - 1 + usize::from(more)))
                .into_iter()
                .collect(),
            subject: format!("Commit {number:04}"),
            author: "Test Author".to_owned(),
            date: "2026-11-08 14:37".to_owned(),
            refs: String::new(),
            graph: GraphRow::default(),
        })
        .collect();
    let lanes = assign_graph(&mut commits, Vec::new());
    let mut state = App::new(Rc::new(I18n::new(Language::English)));
    state.loaded = true;
    state
        .workspace
        .create_space("Work".to_owned(), vec![PathBuf::from("project")]);
    state.panes = pane_grid::State::new(Panel::Git).0;
    state.project_directory = state.workspace.directory().cloned();
    state.repository = Some(Repository {
        remotes: vec!["origin".to_owned()],
        root: PathBuf::from("project"),
        branches: Vec::new(),
        remote: None,
        head: "main".to_owned(),
        next_history: more.then_some(HistoryCursor {
            revisions: vec![id(count + 1)],
            offset: count,
            lanes,
        }),
        commits,
    });
    state
}

#[test]
fn repository_load_focuses_checkout_and_preserves_selected_branches() {
    use crate::domain::repository::Branch;

    for selected in [
        None,
        Some("refs/heads/main"),
        Some("refs/remotes/origin/main"),
    ] {
        let mut state = history_state(2, false);
        state.branch = selected.map(str::to_owned);
        let mut repo = state.repository.take().unwrap();
        repo.head = "work/current".to_owned();
        repo.branches = [
            ("main", "refs/heads/main", false, false),
            ("work/current", "refs/heads/work/current", true, false),
            ("origin/main", "refs/remotes/origin/main", false, true),
        ]
        .into_iter()
        .map(|(name, reference, current, remote)| Branch {
            name: name.to_owned(),
            reference: reference.to_owned(),
            current,
            kind: if remote {
                crate::domain::repository::ReferenceKind::Remote
            } else {
                crate::domain::repository::ReferenceKind::Local
            },
        })
        .collect();
        let _ = state.update(Message::RepositoryLoaded(0, 0, Ok(repo)));

        assert_eq!(
            state.branch.as_deref(),
            Some(selected.unwrap_or("refs/heads/work/current"))
        );
        assert!(state.selected_commit.is_none());
    }
}

#[test]
fn tags_select_history_and_collapse_independently() {
    use crate::domain::repository::{Branch, ReferenceKind};
    let mut state = history_state(3, false);
    state.repository.as_mut().unwrap().branches = [
        ("main", "refs/heads/main", ReferenceKind::Local),
        ("v1.0", "refs/tags/v1.0", ReferenceKind::Tag),
    ]
    .into_iter()
    .map(|(name, reference, kind)| Branch {
        name: name.to_owned(),
        reference: reference.to_owned(),
        current: kind == ReferenceKind::Local,
        kind,
    })
    .collect();
    let _ = state.update(Message::ToggleBranchFolder("tags".to_owned()));
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
    ui.click("v1.0").unwrap();
    assert!(
        matches!(ui.into_messages().collect::<Vec<_>>().as_slice(), [Message::SelectBranch(reference)] if reference == "refs/tags/v1.0")
    );
    let _ = state.update(Message::ToggleBranchFolder("tags".to_owned()));
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
    assert!(ui.find("v1.0").is_err());
    assert!(ui.find("main").is_ok());
}

#[test]
fn git_columns_resize_by_dragging_both_separators() {
    use iced::{Event, mouse};
    let mut state = history_state(3, false);
    let id = state.repository.as_ref().unwrap().commits[0].id.clone();
    let _ = state.update(Message::SelectCommit(id));
    let bounds = |state: &App, id| {
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1440., 920.), state.view());
        ui.find(iced::widget::Id::new(id))
            .unwrap()
            .visible_bounds()
            .unwrap()
    };
    for left in ["git-branches", "git-commits"] {
        let before = bounds(&state, left);
        let edge = iced::Point::new(before.x + before.width + 0.5, before.y + 80.);
        let end = iced::Point::new(edge.x + 65., edge.y);
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1440., 920.), state.view());
        ui.point_at(edge);
        ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position: edge }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        ]);
        ui.point_at(end);
        ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position: end }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, Message::ResizeGit(_)))
        );
        for message in messages {
            let _ = state.update(message);
        }
        assert!(bounds(&state, left).width > before.width + 30.);
        let commits = bounds(&state, "git-commits");
        let search = bounds(&state, "git-search");
        assert!((search.x - commits.x - 12.).abs() < 0.5);
        assert!(search.x + search.width <= commits.x + commits.width);
    }
}

#[test]
fn commit_search_remains_above_history_when_columns_and_window_resize() {
    for width in [1080., 1440.] {
        for with_files in [false, true] {
            for ratio in [0.12, 0.22, 0.88] {
                for loading in [false, true] {
                    let mut state = history_state(12, false);
                    state.i18n = Rc::new(I18n::new(Language::Russian));
                    state.loading = loading;
                    if with_files {
                        let id = state.repository.as_ref().unwrap().commits[0].id.clone();
                        let _ = state.update(Message::SelectCommit(id));
                    }
                    let split = *state.git_panes.layout().splits().next().unwrap();
                    state.git_panes.resize(split, ratio);
                    let mut ui =
                        iced_test::Simulator::with_size(ui_settings(), (width, 720.), state.view());
                    let commits = ui
                        .find(iced::widget::Id::new("git-commits"))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    let search = ui
                        .find(iced::widget::Id::new("git-search"))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    assert!((search.x - commits.x - 12.).abs() < 0.5);
                    assert!(search.y + search.height <= commits.y);
                    assert!(search.x + search.width <= commits.x + commits.width);
                    assert!(search.width >= 72.);
                    if ratio == 0.22
                        && with_files
                        && !loading
                        && let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from)
                    {
                        ui.snapshot(&Theme::Dark)
                            .unwrap()
                            .matches_image(output.join(format!("commit-search-{width}")))
                            .unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn commit_search_input_filters_history_from_the_column_header() {
    let mut state = history_state(3, false);
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
    ui.click(iced::widget::Id::new("git-search")).unwrap();
    ui.typewrite("Commit 0002");
    for message in ui.into_messages() {
        let _ = state.update(message);
    }
    assert_eq!(state.query, "Commit 0002");
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
    assert!(ui.find("Commit 0002").is_ok());
    assert!(ui.find("Commit 0001").is_err());
    assert!(ui.find("Commit 0003").is_err());
}

#[test]
fn history_loading_preserves_rows_and_position_and_rejects_stale_pages() {
    let mut state = history_state(250, true);
    state.selected_commit = Some(state.repository.as_ref().unwrap().commits[0].id.clone());
    let selected = state.selected_commit.clone();
    let _ = state.update(Message::HistoryScrolled {
        offset: 6900.,
        remaining: 100.,
    });
    assert!(state.history_loading);
    let _ = state.update(Message::LoadMoreHistory);
    for (generation, request, offset) in [(1, 0, 250), (0, 1, 250), (0, 0, 0)] {
        let _ = state.update(Message::HistoryLoaded(
            generation,
            request,
            offset,
            Err(Failure::Read),
        ));
        assert!(state.history_loading);
        assert!(state.history_error.is_none());
    }
    let _ = state.update(Message::HistoryLoaded(0, 0, 250, Err(Failure::Read)));
    assert!(!state.history_loading);
    assert_eq!(state.history_error, Some(Failure::Read));
    assert_eq!(state.repository.as_ref().unwrap().commits.len(), 250);
    let _ = state.update(Message::HistoryScrolled {
        offset: 6900.,
        remaining: 50.,
    });
    assert!(
        !state.history_loading,
        "Failed pages require an explicit retry"
    );
    let _ = state.update(Message::LoadMoreHistory);
    assert!(state.history_loading && state.history_error.is_none());
    let page = HistoryPage {
        commits: history_state(1, false).repository.unwrap().commits,
        next: None,
    };
    let _ = state.update(Message::HistoryLoaded(0, 0, 250, Ok(page.clone())));
    assert_eq!(state.repository.as_ref().unwrap().commits.len(), 251);
    assert_eq!(state.history_scroll_y, 6900.);
    assert_eq!(state.selected_commit, selected);
    assert!(!state.history_loading);
    let _ = state.update(Message::HistoryLoaded(0, 0, 250, Ok(page)));
    assert_eq!(state.repository.as_ref().unwrap().commits.len(), 251);
}

#[test]
fn history_search_preloads_pages_and_branch_changes_invalidate_pending_reads() {
    let mut state = history_state(250, true);
    state.history_scroll_y = 5000.;
    let _ = state.update(Message::Search("old commit".to_owned()));
    assert!(state.history_loading);
    assert_eq!(state.history_scroll_y, 0.);
    let mut next = state
        .repository
        .as_ref()
        .unwrap()
        .next_history
        .clone()
        .unwrap();
    next.offset += 1;
    let page = HistoryPage {
        commits: history_state(1, false).repository.unwrap().commits,
        next: Some(next),
    };
    let _ = state.update(Message::HistoryLoaded(0, 0, 250, Ok(page.clone())));
    assert!(
        state.history_loading,
        "Search continues through older pages"
    );
    let _ = state.update(Message::Search(String::new()));
    let _ = state.update(Message::HistoryLoaded(
        0,
        0,
        251,
        Ok(HistoryPage {
            commits: Vec::new(),
            next: page.next,
        }),
    ));
    assert!(
        !state.history_loading,
        "Clearing search stops background preloading"
    );
    let _ = state.update(Message::LoadMoreHistory);
    assert!(state.history_loading);
    let _ = state.update(Message::SelectBranch("refs/heads/other".to_owned()));
    assert!(!state.history_loading);
    assert_eq!(state.history_scroll_y, 0.);
    let _ = state.update(Message::HistoryLoaded(0, 0, 251, Err(Failure::Read)));
    assert!(state.history_error.is_none());
}

#[test]
fn commit_history_scroll_reaches_the_oldest_row_and_requests_more() {
    use iced::{Event, mouse};
    for more in [false, true] {
        let mut state = history_state(1000, more);
        let size = (1080., 720.);
        let bounds = {
            let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
            ui.find(iced::widget::Id::new("commit-history"))
                .unwrap()
                .visible_bounds()
                .unwrap()
        };
        state.history_scroll_y = (28. + 1000. * 30. - bounds.height).max(0.);
        let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
        ui.point_at(bounds.center());
        ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels {
                x: 0.,
                y: -100_000.,
            },
        })]);
        let last = ui.find("Commit 0001").unwrap().visible_bounds().unwrap();
        assert!(
            last.y >= bounds.y && last.y + last.height <= bounds.y + bounds.height - 12.,
            "The oldest row must clear the horizontal scrollbar"
        );
        assert!(
            ui.find("Commit 1000").is_err(),
            "Distant rows must not become widgets"
        );
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.point_at((0., 0.));
            ui.snapshot(&Theme::Dark)
                .unwrap()
                .matches_image(output.join(if more {
                    "history-next-page"
                } else {
                    "history-end"
                }))
                .unwrap();
        }
        for message in ui.into_messages() {
            let _ = state.update(message);
        }
        assert_eq!(state.history_loading, more);
    }
}

#[test]
fn commit_titles_follow_the_graph_in_each_row() {
    use crate::domain::repository::{Commit, GraphRow, assign_graph};

    let id = |number: u32| format!("{number:07x}{:033x}", 0);
    let mut history = vec![
        (1, "Recent change".to_owned(), vec![2]),
        (2, "Merge feature".to_owned(), vec![3, 4]),
        (3, "Update main".to_owned(), vec![5]),
        (4, "Update feature".to_owned(), vec![5]),
        (5, "Shared history".to_owned(), vec![6]),
        (6, "Merge many branches".to_owned(), (7..25).collect()),
    ];
    history.extend((7..25).map(|number| (number, format!("Branch {number}"), vec![25])));
    history.push((25, "Initial commit".to_owned(), Vec::new()));
    let mut commits: Vec<_> = history
        .into_iter()
        .map(|(number, subject, parents)| Commit {
            id: id(number),
            parents: parents.into_iter().map(id).collect(),
            subject,
            author: format!("Author {number}"),
            date: format!("2026-10-{number:02} 09:05"),
            refs: String::new(),
            graph: GraphRow::default(),
        })
        .collect();
    assign_graph(&mut commits, Vec::new());
    let mut state = App::new(Rc::new(I18n::new(Language::English)));
    state.loaded = true;
    state.panes = pane_grid::State::new(Panel::Git).0;
    state.repository = Some(Repository {
        remotes: vec!["origin".to_owned()],
        root: PathBuf::new(),
        branches: Vec::new(),
        commits,
        next_history: None,
        remote: None,
        head: "main".to_owned(),
    });
    for (name, size) in [
        ("history", (1440., 920.)),
        ("history-compact", (1080., 720.)),
    ] {
        let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
        let heading = ui.find("Commit").unwrap().visible_bounds().unwrap();
        let recent = ui.find("Recent change").unwrap().visible_bounds().unwrap();
        let merged = ui.find("Merge feature").unwrap().visible_bounds().unwrap();
        let shared = ui.find("Shared history").unwrap().visible_bounds().unwrap();
        assert!(
            recent.x - heading.x < 32.,
            "Later branches must not indent earlier titles"
        );
        assert!(merged.x > recent.x && merged.x - recent.x < 20.);
        assert_eq!(shared.x, recent.x, "Closed lanes must release title space");
        if size.0 > 1080. {
            for (first, second) in [
                ("Author 1", "Author 2"),
                ("2026-10-01 09:05", "2026-10-02 09:05"),
            ] {
                assert_eq!(
                    ui.find(first).unwrap().visible_bounds().unwrap().x,
                    ui.find(second).unwrap().visible_bounds().unwrap().x,
                );
            }
        }
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            let mut capture = iced_test::Simulator::with_size(ui_settings(), size, state.view());
            capture
                .snapshot(&Theme::Dark)
                .unwrap()
                .matches_image(output.join(name))
                .unwrap();
        }
        ui.click("Recent change").unwrap();
        assert!(matches!(ui.into_messages().collect::<Vec<_>>().as_slice(),
            [Message::SelectCommit(selected)] if selected == &id(1)));
    }
    state.query = "Update".to_owned();
    let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
    let main = ui.find("Update main").unwrap().visible_bounds().unwrap();
    let feature = ui.find("Update feature").unwrap().visible_bounds().unwrap();
    assert_eq!(
        main.x, feature.x,
        "Search results must align after their hashes"
    );
    ui.find("0000003").unwrap();
    ui.find("0000004").unwrap();
}

#[test]
fn sidebar_navigation_fits_and_routes_in_every_screen_and_language() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let app = application(Rc::new(I18n::new(language)));
        let (mut state, _) = app.boot();
        state.loaded = true;
        let settings = Program::window(&app).unwrap();
        for size in [settings.size, settings.min_size.unwrap()] {
            for current in [
                Navigation::Projects,
                Navigation::Files,
                Navigation::Settings,
            ] {
                state.navigation = current;
                let navigation = vec![("nav-settings", Navigation::Settings)];
                for (id, expected) in navigation {
                    let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
                    let bounds = ui
                        .click(iced::widget::Id::new(id))
                        .unwrap()
                        .visible_bounds()
                        .unwrap();
                    assert!(bounds.x >= 0. && bounds.x + bounds.width <= theme::SIDEBAR_WIDTH);
                    assert!(
                        bounds.y >= theme::SIDEBAR_TOP_INSET
                            && bounds.y + bounds.height <= size.height
                    );
                    let messages: Vec<_> = ui.into_messages().collect();
                    assert!(
                        matches!(messages.as_slice(), [Message::Navigate(target)] if *target == expected)
                    );
                }
                let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
                assert!(ui.find(iced::widget::Id::new("nav-projects")).is_err());
                assert!(ui.find(iced::widget::Id::new("nav-files")).is_err());
                ui.click(iced::widget::Id::new("new-space")).unwrap();
                assert!(matches!(
                    ui.into_messages().collect::<Vec<_>>().as_slice(),
                    [Message::Space(spaces::Message::New)]
                ));
            }
        }
    }
}

#[test]
fn welcome_and_settings_fit_supported_window_sizes_in_every_language() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let app = application(Rc::new(I18n::new(language)));
        let (mut state, _) = app.boot();
        let settings = Program::window(&app).unwrap();
        assert!(settings.decorations && settings.exit_on_close_request);
        assert_eq!(settings.transparent, cfg!(target_os = "macos"));
        for size in [settings.size, settings.min_size.unwrap()] {
            for (navigation, key) in [
                (Navigation::Projects, Key::Greeting),
                (Navigation::Settings, Key::SettingsDescription),
            ] {
                state.navigation = navigation;
                let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
                let label = state.i18n.text(key);
                let bounds = ui.find(label.as_str()).unwrap().visible_bounds().unwrap();
                assert!(bounds.x >= 0. && bounds.y >= 0.);
                assert!(bounds.x + bounds.width <= size.width);
                assert!(bounds.y + bounds.height <= size.height);
                let header = ui
                    .find(iced::widget::Id::new("window-header-drag-area"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert_eq!(header.x, 0.);
                assert_eq!(header.y, 0.);
                assert_eq!(header.width, theme::SIDEBAR_WIDTH);
                assert_eq!(header.height, theme::SIDEBAR_TOP_INSET);
            }
        }
    }
}

#[test]
fn header_moves_the_window_without_capturing_action_buttons() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let app = application(Rc::new(I18n::new(language)));
        let (mut state, _) = app.boot();
        let settings = Program::window(&app).unwrap();
        for size in [settings.size, settings.min_size.unwrap()] {
            for navigation in [
                Navigation::Projects,
                Navigation::Files,
                Navigation::Settings,
            ] {
                state.navigation = navigation;
                let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
                ui.click(iced::widget::Id::new("window-header-drag-area"))
                    .unwrap();
                ui.click(iced::widget::Id::new("window-header-drag-area"))
                    .unwrap();
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(matches!(
                    messages.as_slice(),
                    [
                        Message::DragWindow,
                        Message::DragWindow,
                        Message::ToggleWindowMaximized
                    ]
                ));

                let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
                ui.click(iced::widget::Id::new("window-header-action"))
                    .unwrap();
                let messages: Vec<_> = ui.into_messages().collect();
                if navigation == Navigation::Settings {
                    assert!(matches!(
                        messages.as_slice(),
                        [Message::Navigate(Navigation::Projects)]
                    ));
                } else {
                    assert!(matches!(messages.as_slice(), [Message::Refresh]));
                }
            }
        }
    }
}

#[test]
fn background_remains_opaque_until_glass_succeeds() {
    let app = application(Rc::new(I18n::new(Language::English)));
    let (mut state, _) = app.boot();
    for appearance in [Appearance::Dark, Appearance::Light] {
        let _ = state.update(Message::SetAppearance(appearance));
        for (ready, alpha) in [(false, 1.), (true, 0.), (false, 1.)] {
            let _ = state.update(Message::GlassConfigured(ready));
            let style = Program::style(&app, &state, &state.theme());
            assert_eq!(style.background_color.a, alpha);
            assert_eq!(style.text_color, theme::TEXT.resolve(&state.theme()));
            if !ready {
                assert_eq!(
                    Some(style.background_color.into()),
                    theme::surface(&state.theme()).background
                );
            }
        }
    }
}

#[test]
fn stale_file_and_repository_results_cannot_replace_current_selection() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.generation = 4;
    app.file_request = 9;
    app.preview_mut().id = 9;
    app.repo_request = 3;
    app.preview_mut().title = "current.rs".to_owned();
    let _ = app.update(Message::FileLoaded(3, 9, Err(Failure::Binary)));
    let _ = app.update(Message::FileLoaded(4, 8, Err(Failure::Binary)));
    let _ = app.update(Message::RepositoryLoaded(4, 2, Err(Failure::NotRepository)));
    assert!(app.preview().error.is_none());
    assert!(app.repository_error.is_none());
    let _ = app.update(Message::FileLoaded(4, 9, Err(Failure::Binary)));
    assert_eq!(app.preview().error, Some(Failure::Binary));
}

#[test]
fn git_and_pipeline_panels_open_exclusively_and_can_both_close() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.workspace
        .create_space("Work".to_owned(), vec![PathBuf::from("project")]);
    let _ = app.update(Message::TogglePanel(Panel::Pipelines));
    assert!(
        !app.panes
            .iter()
            .any(|(_, panel)| *panel == Panel::Pipelines)
    );
    app.repository = Some(Repository {
        remotes: vec!["origin".to_owned()],
        root: PathBuf::from("project"),
        branches: vec![],
        commits: vec![],
        next_history: None,
        remote: crate::domain::repository::Remote::parse("git@github.com:team/app.git"),
        head: "main".to_owned(),
    });
    app.sync_panels();
    assert_eq!(app.panes.len(), 2);
    assert!(!app.workspace.preferences.show_git);
    assert!(
        app.panes
            .iter()
            .any(|(_, panel)| *panel == Panel::Pipelines)
    );
    let _ = app.update(Message::TogglePanel(Panel::Git));
    assert_eq!(app.panes.len(), 2);
    assert!(app.workspace.preferences.show_git);
    assert!(!app.workspace.preferences.show_pipelines);
    assert!(app.panes.iter().any(|(_, panel)| *panel == Panel::Git));
    assert!(
        !app.panes
            .iter()
            .any(|(_, panel)| *panel == Panel::Pipelines)
    );
    let _ = app.update(Message::TogglePanel(Panel::Pipelines));
    assert!(!app.workspace.preferences.show_git);
    assert!(!app.panes.iter().any(|(_, panel)| *panel == Panel::Git));
    assert!(
        app.panes
            .iter()
            .any(|(_, panel)| *panel == Panel::Pipelines)
    );
    let _ = app.update(Message::TogglePanel(Panel::Pipelines));
    assert_eq!(app.panes.len(), 1);
    assert!(!app.workspace.preferences.show_git);
    assert!(!app.workspace.preferences.show_pipelines);
}

#[test]
fn corrupted_storage_is_not_silently_overwritten() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let _ = app.update(Message::Loaded(Err(Failure::Storage)));
    let _ = app.update(Message::SetCodeSize(18));
    assert!(app.save_blocked);
    assert!(!app.saving);
    assert!(
        app.notifications
            .items
            .iter()
            .any(|item| item.message == Key::StorageLoadFailed)
    );
}

#[test]
fn settings_change_locale_and_queue_serialized_saves() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    let _ = app.update(Message::SetLanguage("es".to_owned()));
    assert_eq!(app.i18n.text(Key::Settings), "Ajustes");
    let _ = app.update(Message::SetCodeSize(19));
    assert!(app.saving && app.save_pending);
    let _ = app.update(Message::Saved(Ok(())));
    assert!(app.saving && !app.save_pending);
}

fn token_dialog() -> auth::Dialog {
    auth::Dialog {
        account: crate::domain::auth::Account {
            provider: Provider::GitLab,
            host: "gitlab.example.com".to_owned(),
        },
        prompt: None,
        token: crate::domain::auth::Secret::default(),
        stage: auth::Stage::Token,
        error: None,
    }
}

#[test]
fn cancelled_authentication_cannot_update_the_selected_account() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.auth_request = 10;
    app.auth_cancel = Some(CancellationToken::new());
    let cancellation = app.auth_cancel.clone().unwrap();
    app.auth_dialog = Some(token_dialog());
    let _ = app.update(Message::Auth(auth::Message::Cancel));
    assert!(cancellation.is_cancelled());
    assert!(app.auth_dialog.is_none());
    let _ = app.update(Message::Auth(auth::Message::Finished(
        10,
        Ok("previous-account".to_owned()),
    )));
    assert!(app.account_username.is_none());
    let _ = app.update(Message::PipelinesLoaded(
        99,
        0,
        pipelines::Loaded {
            runs: Ok(vec![]),
            username: Some("stale".to_owned()),
        },
    ));
    assert!(app.account_username.is_none());
}

#[test]
fn authentication_replaces_an_inflight_pipeline_refresh() {
    let mut app = history_state(1, false);
    app.repository.as_mut().unwrap().remote =
        crate::domain::repository::Remote::parse("https://github.com/team/app.git");
    let _ = app.update(Message::RefreshPipelines);
    let previous = app.pipeline_request;
    assert!(app.pipeline_refreshing);
    let _ = app.update(Message::Auth(auth::Message::Finished(
        app.auth_request,
        Ok("connected".to_owned()),
    )));
    assert!(app.pipeline_request > previous);
    let _ = app.update(Message::PipelinesLoaded(
        app.generation,
        previous,
        pipelines::Loaded {
            runs: Err(Failure::Unavailable),
            username: None,
        },
    ));
    assert!(app.pipeline_refreshing);
    assert_eq!(app.account_username.as_deref(), Some("connected"));
    assert!(matches!(app.pipelines, PipelineState::Loading));
}

#[test]
fn token_submission_clears_the_form_and_hides_tokens_in_messages() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.auth_dialog = Some(token_dialog());
    let message = Message::Auth(auth::Message::TokenChanged(
        crate::domain::auth::Secret::new("private-token".to_owned()),
    ));
    assert!(!format!("{message:?}").contains("private-token"));
    let _ = app.update(message);
    let _ = app.update(Message::Auth(auth::Message::SubmitToken));
    let dialog = app.auth_dialog.as_ref().unwrap();
    assert!(dialog.token.is_empty());
    assert_eq!(dialog.stage, auth::Stage::Connecting);
    let _ = app.update(Message::Auth(auth::Message::Finished(
        app.auth_request,
        Err(Failure::InvalidToken),
    )));
    let dialog = app.auth_dialog.as_ref().unwrap();
    assert_eq!(dialog.stage, auth::Stage::Token);
    assert!(dialog.token.is_empty());
    assert_eq!(dialog.error, Some(Failure::InvalidToken));
}

#[test]
fn escape_without_a_dialog_does_not_interrupt_sign_out() {
    let mut app = App::new(Rc::new(I18n::new(Language::English)));
    app.auth_request = 12;
    app.disconnecting = true;
    let _ = app.update(Message::Auth(auth::Message::Cancel));
    assert_eq!(app.auth_request, 12);
    let _ = app.update(Message::Auth(auth::Message::Disconnected(
        app.generation,
        12,
        Ok(()),
    )));
    assert!(!app.disconnecting);
}

#[test]
fn translated_authentication_dialogs_fit_the_minimum_window() {
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = App::new(Rc::new(I18n::new(language)));
        for stage in [
            auth::Stage::Token,
            auth::Stage::Waiting,
            auth::Stage::Failed,
        ] {
            let mut dialog = token_dialog();
            dialog.stage = stage;
            if stage == auth::Stage::Waiting {
                dialog.prompt = Some(crate::domain::auth::AuthPrompt {
                    url: "https://github.com/login/device".to_owned(),
                    code: Some("ABCD-EFGH".to_owned()),
                });
            } else if stage == auth::Stage::Failed {
                dialog.error = Some(Failure::AuthNotConfigured);
            }
            app.auth_dialog = Some(dialog);
            let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), app.view());
            let label = app.i18n.text(Key::CancelSignIn);
            let bounds = ui.find(label.as_str()).unwrap().visible_bounds().unwrap();
            assert!(bounds.x >= 0. && bounds.y >= 0.);
            assert!(bounds.x + bounds.width <= 1080. && bounds.y + bounds.height <= 720.);
        }
    }
}

#[tokio::test]
async fn render_for_visual_review_when_requested() {
    let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) else {
        return;
    };
    let mut state = App::new(Rc::new(I18n::new(Language::Russian)));
    state.loaded = true;
    let capture = |state: &App, name: &str, size: (f32, f32)| {
        if std::env::var("IWE_RENDER_CASE").is_ok_and(|filter| !name.contains(&filter)) {
            return;
        }
        let mut ui = iced_test::Simulator::with_size(ui_settings(), size, state.view());
        let snapshot = ui.snapshot(&Theme::Dark).unwrap();
        snapshot.matches_image(output.join(name)).unwrap();
    };
    let mut long_history = history_state(20_000, false);
    long_history.i18n = Rc::new(I18n::new(Language::Russian));
    capture(&long_history, "scrollbar-long-history", (1080., 720.));
    let mut long_titles = history_state(4, false);
    long_titles.i18n = Rc::new(I18n::new(Language::Russian));
    for commit in &mut long_titles.repository.as_mut().unwrap().commits {
        commit.subject =
            "Исправлен пересчёт размеров интерфейса при изменении ширины окна и открытии файлов"
                .to_owned();
    }
    capture(&long_titles, "git-columns", (1080., 720.));
    capture(&state, "welcome", (1440., 920.));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut opened = App::new(Rc::new(I18n::new(Language::Russian)));
    opened
        .workspace
        .create_space("Work".to_owned(), vec![root.clone()]);
    opened.loaded = true;
    opened.navigation = Navigation::Files;
    for filename in [
        "Cargo.toml",
        "src/app/mod.rs",
        "src/ui/screens/workspace.rs",
    ] {
        let path = root.join(filename);
        let _ = opened.open_file(path.clone());
        let document = Document::new(
            std::fs::read_to_string(&path).unwrap(),
            &crate::ui::code::syntax_for(&path),
        );
        let _ = opened.finish_preview(opened.preview().id, Ok(document));
    }
    capture(&opened, "file-tabs", (1440., 920.));
    state
        .workspace
        .create_space("Work".to_owned(), vec![root.clone()]);
    let repo = repository::load(root.clone(), None).await.unwrap();
    if let Some(commit) = repo.commits.first() {
        state.selected_commit = Some(commit.id.clone());
        state.changed_files = repository::changed_files(root.clone(), commit.clone())
            .await
            .unwrap();
    }
    state.repository = Some(repo);
    if state.selected_commit.is_some() {
        state.show_commit_files();
    }
    state.tree.insert(
        root.clone(),
        workspace::entries(root.clone(), root.clone())
            .await
            .unwrap(),
    );
    state.expanded.insert(root.clone());
    state.preview_mut().title = "src/app/mod.rs".to_owned();
    state.preview_mut().document = Some(
        Document::new(
            std::fs::read_to_string(root.join("src/app/mod.rs")).unwrap(),
            "rs",
        )
        .into(),
    );
    state.workspace.preferences.show_pipelines = false;
    state.pipelines = PipelineState::Ready(Vec::new());
    state.sync_panels();
    capture(&state, "workspace", (1440., 920.));
    state.navigation = Navigation::Files;
    capture(&state, "files", (1080., 720.));
    state.navigation = Navigation::Settings;
    for (language, name) in [
        (Language::English, "settings-en"),
        (Language::Russian, "settings-ru"),
        (Language::Spanish, "settings-es"),
    ] {
        state.i18n = Rc::new(I18n::new(language));
        capture(&state, name, (1080., 720.));
    }
    state.navigation = Navigation::Files;
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        state.i18n = Rc::new(I18n::new(language));
        state.auth_dialog = Some(token_dialog());
        capture(&state, &format!("auth-token-{suffix}"), (1080., 720.));
        let dialog = state.auth_dialog.as_mut().unwrap();
        dialog.account = crate::domain::auth::Account {
            provider: Provider::GitHub,
            host: "github.com".to_owned(),
        };
        dialog.stage = auth::Stage::Waiting;
        dialog.prompt = Some(crate::domain::auth::AuthPrompt {
            url: "https://github.com/login/device".to_owned(),
            code: Some("ABCD-EFGH".to_owned()),
        });
        capture(&state, &format!("auth-browser-{suffix}"), (1080., 720.));
    }

    let mut state = App::new(Rc::new(I18n::new(Language::Russian)));
    state.loaded = true;
    state.navigation = Navigation::Files;
    let root = PathBuf::from("/workspace/demo");
    state
        .workspace
        .create_space("Work".to_owned(), vec![root.clone()]);
    let entries = |directory: &std::path::Path, names: &[(&str, bool)]| {
        names
            .iter()
            .map(|(name, directory_entry)| FileEntry {
                path: directory.join(name),
                name: (*name).to_owned(),
                directory: *directory_entry,
            })
            .collect()
    };
    state.tree.insert(
        root.clone(),
        entries(
            &root,
            &[
                ("config", true),
                ("deploy", true),
                ("docs", true),
                ("src", true),
                (".gitignore", false),
                (".gitlab-ci.yml", false),
                (".pre-commit-config.yaml", false),
                ("Dockerfile", false),
                ("docker-compose.yml", false),
                ("Makefile", false),
                ("package.json", false),
                ("poetry.lock", false),
                ("pyproject.toml", false),
                ("README.md", false),
            ],
        ),
    );
    let src = root.join("src");
    state.tree.insert(
        src.clone(),
        entries(
            &src,
            &[
                ("apps", true),
                ("services", true),
                ("settings", true),
                ("__init__.py", false),
                ("celery_app.py", false),
                ("env.py", false),
                ("manage.py", false),
                ("urls.py", false),
                ("App.tsx", false),
                ("Button.vue", false),
                ("main.ts", false),
                ("styles.scss", false),
            ],
        ),
    );
    state.expanded.extend([root.clone(), src.clone()]);
    state.files.selected_path = Some(src);
    state.preview_mut().title = "src/celery_app.py".to_owned();
    state.preview_mut().document = Some(Document::new(
        "from __future__ import absolute_import, unicode_literals\n\nfrom services.celery import app as celery_app\n# Initialize Celery signals\nfrom celery import signals\nimport extensions.request_id.signals\n\n\n@signals.task_postrun.connect\ndef clean_client_connect(*args, **kwargs) -> None:\n    from django.apps import apps\n\n    app_config = apps.get_app_config(\"client\")\n\n    if hasattr(app_config, \"ready\"):\n        app_config.reset_context_caches()\n\n\n__all__ = ('celery_app',)\n".to_owned(), "py",
    ).into());
    capture(&state, "ide-python", (1440., 1000.));
    state.files.selected_path = Some(root.join("src/App.tsx"));
    state.preview_mut().title = "src/App.tsx".to_owned();
    state.preview_mut().document = Some(Document::new(
        "import { useState } from 'react';\n\ninterface Project {\n  id: number;\n  name: string;\n}\n\n// Show the selected project\nexport function App() {\n  const [project, setProject] = useState<Project | null>(null);\n  const count = 42;\n\n  return (\n    <section className=\"workspace\">\n      <h1>{project?.name ?? 'Choose a project'}</h1>\n      <button onClick={() => setProject({ id: count, name: 'Demo' })}>\n        Open project\n      </button>\n    </section>\n  );\n}\n".to_owned(), "tsx",
    ).into());
    capture(&state, "ide-web", (1080., 720.));

    let previous = state.preview().document.as_ref().unwrap().text.clone();
    let current = previous
        .replace(
            "const count = 42;",
            "const count = 128;\n  const ready = true;",
        )
        .replace("Open project", "Create project");
    state.preview_mut().document = Some(
        Document::from_source(
            crate::domain::changes::Source {
                text: current,
                previous: Some(previous),
                ..Default::default()
            },
            "tsx",
        )
        .into(),
    );
    state.changes.status = crate::domain::changes::Worktree::new(
        &root,
        [
            ("src/App.tsx", crate::domain::changes::FileStatus::Modified),
            (
                "src/Button.vue",
                crate::domain::changes::FileStatus::Untracked,
            ),
            ("src/styles.scss", crate::domain::changes::FileStatus::Added),
            ("README.md", crate::domain::changes::FileStatus::Modified),
        ]
        .into_iter()
        .map(|(path, status)| {
            (
                root.join(path),
                crate::domain::changes::FileChange {
                    status,
                    original: None,
                },
            )
        })
        .collect(),
    );
    capture(&state, "working-changes", (1440., 1000.));
    state.preview_mut().revision = Some("0123456789abcdef0123456789abcdef01234567".to_owned());
    capture(&state, "commit-changes", (1440., 1000.));

    let mut state = history_state(3, false);
    state.i18n = Rc::new(I18n::new(Language::Russian));
    state.repository.as_mut().unwrap().branches = [
        ("backup/old-feature", false),
        ("dev", false),
        ("origin/dev", true),
        ("upstream/main", true),
    ]
    .into_iter()
    .map(|(name, remote)| crate::domain::repository::Branch {
        name: name.to_owned(),
        reference: format!("refs/{}/{name}", if remote { "remotes" } else { "heads" }),
        current: name == "dev",
        kind: if remote {
            crate::domain::repository::ReferenceKind::Remote
        } else {
            crate::domain::repository::ReferenceKind::Local
        },
    })
    .collect();
    let mut repo = state.repository.take().unwrap();
    for name in ["v1.0", "v2.0"] {
        repo.branches.push(crate::domain::repository::Branch {
            name: name.to_owned(),
            reference: format!("refs/tags/{name}"),
            current: false,
            kind: crate::domain::repository::ReferenceKind::Tag,
        });
    }
    repo.head = "dev".to_owned();
    let _ = state.update(Message::RepositoryLoaded(0, 0, Ok(repo)));
    capture(&state, "branches-checkout", (1080., 720.));
    for (key, name) in [
        (Key::LocalBranches, "branches-local-collapsed"),
        (Key::RemoteBranches, "branches-remote-collapsed"),
        (Key::Tags, "branches-tags-collapsed"),
    ] {
        state.expanded_branch_folders = [
            "false",
            "false/backup",
            "true",
            "true/origin",
            "true/upstream",
            "tags",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let mut ui = iced_test::Simulator::with_size(ui_settings(), (1080., 720.), state.view());
        ui.click(state.i18n.text(key)).unwrap();
        for message in ui.into_messages() {
            let _ = state.update(message);
        }
        capture(&state, name, (1080., 720.));
    }
}
