use super::*;
use crate::{
    domain::workspace::Workspace,
    i18n::{I18n, Language},
    ui::theme,
};
use std::rc::Rc;

fn state(language: Language) -> App {
    let mut app = App::new(Rc::new(I18n::new(language)));
    app.loaded = true;
    app
}

#[test]
fn file_tool_order_changes_by_dragging_and_selects_the_first_tool() {
    use crate::domain::workspace::FilePanel;
    use iced::{Event, mouse, widget::Id};
    let mut app = state(Language::English);
    app.workspace
        .create_space("Work".into(), vec![PathBuf::from("code")]);
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
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
    ui.point_at(first.center());
    ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    let end = iced::Point::new(second.center_x(), second.y + second.height - 2.);
    ui.point_at(end);
    ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position: end }),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    let messages = ui.into_messages().collect::<Vec<_>>();
    assert!(matches!(
        messages.as_slice(),
        [AppMessage::MoveFilePanel(0, 1)]
    ));
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(app.file_panel, FilePanel::Changes);
    assert_eq!(
        app.workspace.preferences.file_panels,
        [FilePanel::Changes, FilePanel::Project]
    );
    let mut restored: Workspace =
        serde_json::from_value(serde_json::to_value(&app.workspace).unwrap()).unwrap();
    restored.normalize();
    let mut next = state(Language::English);
    let _ = next.update(AppMessage::Loaded(Ok((restored, None))));
    assert_eq!(next.file_panel, FilePanel::Changes);
}

#[test]
fn project_context_menu_pins_above_spaces_and_persists_without_duplicates() {
    use iced::{Event, mouse, widget::Id};
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = state(language);
        app.workspace.create_space(
            "Team".into(),
            vec![PathBuf::from("api"), PathBuf::from("web")],
        );
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let bounds = ui
            .find(Id::new("space-project-0-1"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        ui.point_at(bounds.center());
        ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Right,
        ))]);
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.project_menu.is_some());
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(Id::new("toggle-project-pin")).unwrap();
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.workspace.spaces[0].projects[1].pinned);
        assert_eq!(app.workspace.directory(), Some(&PathBuf::from("api")));
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let pinned = ui
            .find(Id::new("space-project-0-1"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let group = ui
            .find(Id::new("space-group-0"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(pinned.y < group.y);
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("pinned-projects-{suffix}")))
                .unwrap();
        }
        drop(ui);
        let restored: Workspace =
            serde_json::from_value(serde_json::to_value(&app.workspace).unwrap()).unwrap();
        assert!(restored.spaces[0].projects[1].pinned);
        let _ = app.update(AppMessage::ProjectMenu(
            0,
            PathBuf::from("web"),
            iced::Point::ORIGIN,
        ));
        let _ = app.update(AppMessage::TogglePinned);
        assert!(!app.workspace.spaces[0].projects[1].pinned);
    }
}

#[test]
fn multiple_folder_selections_append_unique_directories_in_order() {
    let mut app = state(Language::English);
    let _ = app.update_space(Message::New);
    for directories in [vec!["api", "web", "api"], vec!["web", "mobile"], Vec::new()] {
        let _ = app.update_space(Message::PickProject(None));
        let _ = app.update_space(Message::ProjectsPicked(
            app.space_request,
            None,
            directories.into_iter().map(PathBuf::from).collect(),
        ));
        assert!(!app.picking);
    }
    assert_eq!(
        app.space_draft.as_ref().unwrap().directories,
        [
            PathBuf::from("api"),
            PathBuf::from("web"),
            PathBuf::from("mobile")
        ]
    );
    assert!(app.workspace.spaces.is_empty());
    let _ = app.update_space(Message::NameChanged("Team".to_owned()));
    let _ = app.update_space(Message::Create);
    let _ = app.update_space(Message::PickProject(Some(0)));
    let _ = app.update_space(Message::ProjectsPicked(
        app.space_request,
        Some(0),
        vec![
            PathBuf::from("web"),
            PathBuf::from("docs"),
            PathBuf::from("tools"),
        ],
    ));
    assert_eq!(
        app.workspace.spaces[0]
            .projects
            .iter()
            .map(|project| project.name.as_str())
            .collect::<Vec<_>>(),
        ["api", "web", "mobile", "docs", "tools"]
    );
}

#[test]
fn draft_order_saves_only_on_confirmation_and_preserves_the_open_project() {
    let mut app = state(Language::English);
    app.workspace.create_space(
        "Team".to_owned(),
        ["api", "web", "docs"].map(PathBuf::from).to_vec(),
    );
    app.workspace.select_project(0, 1);
    let before = serde_json::to_value(&app.workspace).unwrap();
    let generation = app.generation;
    let _ = app.update_space(Message::Edit(0));
    let _ = app.update_space(Message::MoveProject(None, 1, 0));
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    let _ = app.update_space(Message::Cancel);
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    let _ = app.update_space(Message::Edit(0));
    let _ = app.update_space(Message::MoveProject(None, 1, 0));
    let _ = app.update_space(Message::Create);
    assert_eq!(app.workspace.spaces[0].projects[0].name, "web");
    assert_eq!(app.workspace.selected_project, Some(0));
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("web")));
    assert_eq!(app.generation, generation);
    assert!(app.saving);
}

#[test]
fn sidebar_order_saves_without_reloading_or_switching_projects() {
    let mut app = state(Language::English);
    app.workspace.create_space(
        "Team".to_owned(),
        ["api", "web", "docs"].map(PathBuf::from).to_vec(),
    );
    app.workspace.select_project(0, 1);
    let generation = app.generation;
    let _ = app.update_space(Message::MoveProject(Some(0), 2, 0));
    assert_eq!(app.workspace.spaces[0].projects[0].name, "docs");
    assert_eq!(app.workspace.selected_project, Some(2));
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("web")));
    assert_eq!(app.generation, generation);
    assert!(app.saving);
    let before = serde_json::to_value(&app.workspace).unwrap();
    let _ = app.update_space(Message::Edit(0));
    let _ = app.update_space(Message::MoveProject(Some(0), 0, 2));
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    let _ = app.update_space(Message::PickProject(None));
    let draft = app.space_draft.as_ref().unwrap().directories.clone();
    let _ = app.update_space(Message::MoveProject(None, 0, 2));
    assert_eq!(app.space_draft.as_ref().unwrap().directories, draft);
}

#[test]
fn dragging_in_the_form_and_sidebar_emits_the_correct_move() {
    use iced::{Event, Point, mouse, widget::Id};
    for editing in [false, true] {
        let mut app = state(Language::English);
        app.workspace.create_space(
            "Team".to_owned(),
            ["api", "web", "docs"].map(PathBuf::from).to_vec(),
        );
        if editing {
            let _ = app.update_space(Message::Edit(0));
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let prefix = if editing {
            "space-draft-project-"
        } else {
            "space-project-0-"
        };
        let first = ui
            .find(Id::from(format!("{prefix}0")))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let third = ui
            .find(Id::from(format!("{prefix}2")))
            .unwrap()
            .visible_bounds()
            .unwrap();
        ui.point_at(first.center());
        ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        let end = Point::new(third.center_x(), third.y + third.height - 4.);
        ui.point_at(end);
        ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position: end }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]);
        let messages = ui.into_messages().collect::<Vec<_>>();
        assert!(
            matches!(messages.as_slice(), [AppMessage::Space(Message::MoveProject(space, 0, 2))] if *space == if editing { None } else { Some(0) }),
            "{messages:?}"
        );
        for message in messages {
            let _ = app.update(message);
        }
        if editing {
            assert_eq!(
                app.space_draft.as_ref().unwrap().directories[2],
                PathBuf::from("api")
            );
            assert_eq!(app.workspace.spaces[0].projects[0].name, "api");
        } else {
            assert_eq!(app.workspace.spaces[0].projects[2].name, "api");
            assert_eq!(app.workspace.directory(), Some(&PathBuf::from("api")));
        }
    }
}

#[test]
fn removing_a_project_remains_available_beside_draggable_rows() {
    let mut app = state(Language::English);
    app.workspace
        .create_space("Team".to_owned(), vec![PathBuf::from("api")]);
    let _ = app.update_space(Message::Edit(0));
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    ui.click(iced::widget::Id::new("space-remove-project-0"))
        .unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>().as_slice(),
        [AppMessage::Space(Message::RemoveProject(0))]
    ));
}

fn settings() -> iced::Settings {
    iced::Settings {
        default_font: theme::font(),
        ..Default::default()
    }
}

#[test]
fn spaces_collapse_independently_without_changing_the_open_project() {
    let mut app = state(Language::English);
    app.workspace
        .create_space("Team".to_owned(), vec![PathBuf::from("frontend")]);
    app.workspace
        .create_space("Personal".to_owned(), vec![PathBuf::from("notes")]);
    app.workspace.select_project(0, 0);
    let _ = app.update(AppMessage::ToggleSpace(0));
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    assert!(ui.find(iced::widget::Id::new("space-project-0-0")).is_err());
    assert!(ui.find(iced::widget::Id::new("space-project-1-0")).is_ok());
    assert!(ui.find(iced::widget::Id::new("edit-space-0")).is_ok());
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("frontend")));
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        ui.snapshot(&iced::Theme::Dark)
            .unwrap()
            .matches_image(output.join("spaces-collapsed"))
            .unwrap();
    }
    drop(ui);
    let _ = app.update(AppMessage::ToggleSpace(0));
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    assert!(ui.find(iced::widget::Id::new("space-project-0-0")).is_ok());
}

#[test]
fn editing_a_space_preserves_the_active_project_and_applies_changes_on_save() {
    let mut app = state(Language::English);
    app.workspace.create_space(
        "Team".to_owned(),
        vec![PathBuf::from("frontend"), PathBuf::from("backend")],
    );
    app.workspace.select_project(0, 1);
    app.workspace.spaces[0].projects[1].name = "API".to_owned();
    app.navigation = Navigation::Files;
    let before = serde_json::to_value(&app.workspace).unwrap();
    let generation = app.generation;
    let _ = app.update_space(Message::Edit(0));
    assert_eq!(app.space_draft.as_ref().unwrap().name, "Team");
    let _ = app.update_space(Message::NameChanged(" Platform ".to_owned()));
    let _ = app.update_space(Message::RemoveProject(0));
    let _ = app.update_space(Message::PickProject(None));
    let _ = app.update_space(Message::ProjectsPicked(
        app.space_request,
        None,
        vec![PathBuf::from("mobile")],
    ));
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    let _ = app.update_space(Message::Create);
    assert_eq!(app.workspace.spaces.len(), 1);
    assert_eq!(app.workspace.spaces[0].name, "Platform");
    assert_eq!(app.workspace.spaces[0].projects.len(), 2);
    assert_eq!(app.workspace.spaces[0].projects[0].name, "API");
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("backend")));
    assert_eq!(app.workspace.selected_project, Some(0));
    assert_eq!(app.generation, generation);
    assert_eq!(app.navigation, Navigation::Files);
    assert!(app.saving);
}

#[test]
fn cancel_editing_discards_changes_and_empty_names_do_not_save() {
    let mut app = state(Language::English);
    app.workspace
        .create_space("Team".to_owned(), vec![PathBuf::from("code")]);
    let before = serde_json::to_value(&app.workspace).unwrap();
    let _ = app.update_space(Message::Edit(0));
    let _ = app.update_space(Message::NameChanged("  ".to_owned()));
    let _ = app.update_space(Message::RemoveProject(0));
    let _ = app.update_space(Message::Create);
    assert!(app.space_draft.is_some());
    let _ = app.update_space(Message::Cancel);
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    assert!(!app.saving);
}

#[test]
fn edit_space_controls_and_translations_fit() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = state(language);
        app.workspace.create_space(
            "Team".to_owned(),
            (0..12)
                .map(|index| PathBuf::from(format!("/projects/project-{index}")))
                .collect(),
        );
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(iced::widget::Id::new("edit-space-0")).unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [AppMessage::Space(Message::Edit(0))]
        ));
        let _ = app.update(messages.into_iter().next().unwrap());
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let label = app.i18n.text(crate::i18n::Key::SaveChanges);
        let bounds = ui.find(label.as_str()).unwrap().visible_bounds().unwrap();
        assert!(
            bounds.x >= 0. && bounds.x + bounds.width <= 1080. && bounds.y + bounds.height <= 720.
        );
        for index in 0..5 {
            let row = ui
                .find(iced::widget::Id::from(format!(
                    "space-draft-project-{index}"
                )))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(
                (row.height - theme::SPACE_PROJECT_HEIGHT).abs() < 0.1,
                "row {index}: {row:?}"
            );
        }
        let sixth = ui
            .find(iced::widget::Id::new("space-draft-project-5"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        assert!(
            (sixth.height - theme::SPACE_PROJECT_HEIGHT / 2.).abs() < 0.1,
            "{sixth:?}"
        );
        assert!(
            ui.find(iced::widget::Id::new("space-draft-project-6"))
                .unwrap()
                .visible_bounds()
                .is_none()
        );
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("edit-space-{suffix}")))
                .unwrap();
        }
    }
}

#[test]
fn creating_a_space_names_the_group_and_attaches_only_chosen_projects() {
    let mut app = state(Language::English);
    let _ = app.update_space(Message::New);
    let _ = app.update_space(Message::Create);
    assert!(app.workspace.spaces.is_empty());
    let _ = app.update_space(Message::NameChanged(" Team ".to_owned()));
    for path in ["frontend", "frontend", "backend"] {
        let _ = app.update_space(Message::PickProject(None));
        let request = app.space_request;
        let _ = app.update_space(Message::ProjectsPicked(
            request,
            None,
            vec![PathBuf::from(path)],
        ));
    }
    assert_eq!(app.space_draft.as_ref().unwrap().directories.len(), 2);
    assert!(app.workspace.spaces.is_empty());
    let _ = app.update_space(Message::RemoveProject(0));
    let _ = app.update_space(Message::Create);
    assert!(app.space_draft.is_none());
    assert_eq!(app.workspace.spaces[0].name, "Team");
    assert_eq!(app.workspace.spaces[0].projects.len(), 1);
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("backend")));
    let _ = app.update_space(Message::PickProject(Some(0)));
    let request = app.space_request;
    let _ = app.update_space(Message::ProjectsPicked(
        request,
        Some(0),
        vec![PathBuf::from("frontend")],
    ));
    assert_eq!(app.workspace.spaces[0].projects.len(), 2);
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("frontend")));
}

#[test]
fn cancelling_a_draft_ignores_old_folder_pickers_and_preserves_the_workspace() {
    let mut app = state(Language::English);
    app.workspace
        .create_space("Existing".to_owned(), vec![PathBuf::from("code")]);
    let before = serde_json::to_value(&app.workspace).unwrap();
    let _ = app.update_space(Message::New);
    let _ = app.update_space(Message::NameChanged("Discarded".to_owned()));
    let _ = app.update_space(Message::PickProject(None));
    let request = app.space_request;
    let _ = app.update(AppMessage::DismissDialog);
    assert!(!app.picking && app.space_draft.is_none());
    let _ = app.update_space(Message::New);
    let _ = app.update_space(Message::ProjectsPicked(
        request,
        None,
        vec![PathBuf::from("stale"), PathBuf::from("also-stale")],
    ));
    assert!(app.space_draft.as_ref().unwrap().directories.is_empty());
    let _ = app.update(AppMessage::Navigate(Navigation::Settings));
    assert_eq!(app.navigation, Navigation::Projects);
    let generation = app.generation;
    let _ = app.update(AppMessage::Refresh);
    assert_eq!(app.generation, generation);
    let _ = app.update_space(Message::Cancel);
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
    assert!(!app.saving);
}

#[test]
fn an_unassigned_startup_folder_opens_a_draft_instead_of_creating_a_space() {
    let mut app = state(Language::English);
    let _ = app.update(AppMessage::Loaded(Ok((
        Workspace::default(),
        Some(PathBuf::from("code")),
    ))));
    assert!(app.workspace.spaces.is_empty());
    let draft = app.space_draft.as_ref().unwrap();
    assert!(draft.name.is_empty());
    assert_eq!(draft.directories, [PathBuf::from("code")]);
}

#[test]
fn space_headings_do_not_open_projects() {
    let mut app = state(Language::English);
    app.workspace.create_space(
        "Team".to_owned(),
        vec![PathBuf::from("frontend"), PathBuf::from("backend")],
    );
    app.workspace
        .create_space("Personal".to_owned(), Vec::new());
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    ui.click(iced::widget::Id::new("space-group-0")).unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>().as_slice(),
        [AppMessage::ToggleSpace(0)]
    ));
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
    ui.click(iced::widget::Id::new("space-project-0-1"))
        .unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>().as_slice(),
        [AppMessage::SelectProject(0, 1)]
    ));
    if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.snapshot(&iced::Theme::Dark)
            .unwrap()
            .matches_image(output.join("spaces-sidebar"))
            .unwrap();
    }
}

#[test]
fn translated_space_forms_fit_and_block_the_background() {
    for (language, suffix) in [
        (Language::English, "en"),
        (Language::Russian, "ru"),
        (Language::Spanish, "es"),
    ] {
        let mut app = state(language);
        let _ = app.update_space(Message::New);
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(iced::widget::Id::new("space-create")).unwrap();
        ui.click(iced::widget::Id::new("nav-settings")).unwrap();
        assert!(ui.into_messages().next().is_none());
        let _ = app.update_space(Message::NameChanged("Product team".to_owned()));
        app.space_draft.as_mut().unwrap().directories = (0..12)
            .map(|index| PathBuf::from(format!("/projects/team/project-{index}")))
            .collect();
        for id in [
            "space-name",
            "space-add-project",
            "space-create",
            "space-cancel",
        ] {
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            let bounds = ui
                .find(iced::widget::Id::new(id))
                .unwrap()
                .visible_bounds()
                .unwrap();
            assert!(
                bounds.x >= 0.
                    && bounds.y >= 0.
                    && bounds.x + bounds.width <= 1080.
                    && bounds.y + bounds.height <= 720.
            );
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        ui.click(iced::widget::Id::new("space-create")).unwrap();
        assert!(matches!(
            ui.into_messages().collect::<Vec<_>>().as_slice(),
            [AppMessage::Space(Message::Create)]
        ));
        if let Some(output) = std::env::var_os("IWE_RENDER_DIR").map(PathBuf::from) {
            let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
            ui.snapshot(&iced::Theme::Dark)
                .unwrap()
                .matches_image(output.join(format!("space-form-{suffix}")))
                .unwrap();
        }
    }
}

#[test]
fn space_context_menu_and_edit_form_delete_only_workspace_metadata() {
    use iced::{Event, mouse, widget::Id};
    for language in [Language::English, Language::Russian, Language::Spanish] {
        for from_form in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let keep = directory.path().join("keep");
            let remove = directory.path().join("remove");
            std::fs::create_dir_all(&keep).unwrap();
            std::fs::create_dir_all(&remove).unwrap();
            std::fs::write(remove.join("source.rs"), "fn main() {}\n").unwrap();
            let mut app = state(language);
            app.workspace
                .create_space("Keep".into(), vec![keep.clone()]);
            app.workspace
                .create_space("Remove".into(), vec![remove.clone()]);
            app.collapsed_spaces.insert(0);
            app.collapsed_spaces.insert(1);
            let mut ui = iced_test::Simulator::with_size(settings(), (1280., 900.), app.view());
            let heading = ui
                .find(Id::new("space-group-1"))
                .unwrap()
                .visible_bounds()
                .unwrap();
            ui.point_at(heading.center());
            ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Right,
            ))]);
            for message in ui.into_messages() {
                let _ = app.update(message);
            }
            assert_eq!(app.space_menu.map(|(index, _)| index), Some(1));
            assert!(app.collapsed_spaces.contains(&1));
            let mut ui = iced_test::Simulator::with_size(settings(), (1280., 900.), app.view());
            assert!(ui.find(Id::new("space-menu-delete")).is_ok());
            assert!(ui.find(Id::new("space-menu-edit")).is_ok());
            ui.click(Id::new(if from_form {
                "space-menu-edit"
            } else {
                "space-menu-delete"
            }))
            .unwrap();
            for message in ui.into_messages() {
                let _ = app.update(message);
            }
            if from_form {
                assert!(app.space_menu.is_none());
                assert_eq!(app.space_draft.as_ref().unwrap().editing, Some(1));
                let mut ui = iced_test::Simulator::with_size(settings(), (1280., 900.), app.view());
                let delete = ui
                    .find(Id::new("space-delete"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                let cancel = ui
                    .find(Id::new("space-cancel"))
                    .unwrap()
                    .visible_bounds()
                    .unwrap();
                assert!(delete.x + delete.width < cancel.x);
                ui.click(Id::new("space-delete")).unwrap();
                for message in ui.into_messages() {
                    let _ = app.update(message);
                }
            }
            assert_eq!(app.workspace.spaces.len(), 1);
            assert_eq!(app.workspace.spaces[0].name, "Keep");
            assert_eq!(app.workspace.directory(), Some(&keep));
            assert!(app.space_draft.is_none());
            assert!(app.space_menu.is_none());
            assert_eq!(
                app.collapsed_spaces.iter().copied().collect::<Vec<_>>(),
                vec![0]
            );
            assert_eq!(
                std::fs::read_to_string(remove.join("source.rs")).unwrap(),
                "fn main() {}\n"
            );
            let restored: Workspace =
                serde_json::from_value(serde_json::to_value(&app.workspace).unwrap()).unwrap();
            assert_eq!(restored.spaces.len(), 1);
            assert_eq!(restored.directory(), Some(&keep));
        }
    }
}

#[test]
fn space_deletion_rejects_pending_actions_and_preserves_another_active_project() {
    let mut app = state(Language::English);
    app.workspace.create_space("Remove".into(), Vec::new());
    app.workspace
        .create_space("Keep".into(), vec![PathBuf::from("keep")]);
    app.collapsed_spaces.insert(1);
    app.git_actions.pending = true;
    let _ = app.update_space(Message::Delete(0));
    assert_eq!(app.workspace.spaces.len(), 2);
    app.git_actions.pending = false;
    let _ = app.update_space(Message::Delete(0));
    assert_eq!(app.workspace.selected_space, Some(0));
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("keep")));
    assert!(app.collapsed_spaces.contains(&0));
    let _ = app.update_space(Message::Delete(0));
    assert!(app.workspace.spaces.is_empty());
    assert!(app.workspace.directory().is_none());
    assert!(app.collapsed_spaces.is_empty());
}

#[test]
fn overflowing_space_list_keeps_edit_buttons_outside_the_scrollbar_lane() {
    use iced::widget::Id;
    for language in [Language::English, Language::Russian, Language::Spanish] {
        let mut app = state(language);
        for index in 0..12 {
            app.workspace.create_space(
                format!("Space {index}"),
                vec![PathBuf::from(format!("project-{index}"))],
            );
        }
        let mut ui = iced_test::Simulator::with_size(settings(), (1080., 720.), app.view());
        let edit = ui
            .find(Id::new("edit-space-0"))
            .unwrap()
            .visible_bounds()
            .unwrap();
        let scrollbar_left = theme::SIDEBAR_WIDTH - 12. - 10.;
        assert!(edit.x + edit.width <= scrollbar_left - theme::SIDEBAR_SCROLL_GAP);
        ui.click(Id::new("edit-space-0")).unwrap();
        assert!(
            ui.into_messages()
                .any(|message| matches!(message, AppMessage::Space(Message::Edit(0))))
        );
    }
}

#[test]
fn dragging_spaces_preserves_selection_collapsed_state_and_saved_order() {
    use iced::{Event, Point, mouse, widget::Id};
    let mut app = state(Language::English);
    app.workspace
        .create_space("First".into(), vec!["first".into()]);
    app.workspace
        .create_space("Second".into(), vec!["second".into()]);
    app.workspace
        .create_space("Third".into(), vec!["third".into()]);
    app.workspace.select_project(0, 0);
    app.collapsed_spaces.insert(0);
    let generation = app.generation;
    let mut ui = iced_test::Simulator::with_size(settings(), (1080., 920.), app.view());
    let first = ui
        .find(Id::new("space-group-0"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    let third = ui
        .find(Id::new("space-group-2"))
        .unwrap()
        .visible_bounds()
        .unwrap();
    ui.point_at(first.center());
    ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    // Move past the last group's midpoint, including its project rows.
    let end = Point::new(third.center_x(), third.y + 110.);
    ui.point_at(end);
    ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position: end }),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    let messages = ui.into_messages().collect::<Vec<_>>();
    assert!(
        matches!(
            messages.as_slice(),
            [AppMessage::Space(Message::MoveSpace(0, 2))]
        ),
        "{messages:?}"
    );
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(app.workspace.selected_space, Some(2));
    assert_eq!(app.workspace.directory(), Some(&PathBuf::from("first")));
    assert!(app.collapsed_spaces.contains(&2));
    assert_eq!(app.generation, generation);
    assert!(app.saving);
    let restored: Workspace =
        serde_json::from_value(serde_json::to_value(&app.workspace).unwrap()).unwrap();
    assert_eq!(
        restored
            .spaces
            .iter()
            .map(|space| space.name.as_str())
            .collect::<Vec<_>>(),
        ["Second", "Third", "First"]
    );
    assert_eq!(restored.directory(), Some(&PathBuf::from("first")));
    let before = serde_json::to_value(&app.workspace).unwrap();
    let _ = app.update_space(Message::MoveSpace(99, 0));
    assert_eq!(serde_json::to_value(&app.workspace).unwrap(), before);
}
