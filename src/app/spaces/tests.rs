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
    let _ = app.update_space(Message::ProjectPicked(
        app.space_request,
        None,
        Some(PathBuf::from("mobile")),
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
        let _ = app.update_space(Message::ProjectPicked(
            request,
            None,
            Some(PathBuf::from(path)),
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
    let _ = app.update_space(Message::ProjectPicked(
        request,
        Some(0),
        Some(PathBuf::from("frontend")),
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
    let _ = app.update_space(Message::ProjectPicked(
        request,
        None,
        Some(PathBuf::from("stale")),
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
