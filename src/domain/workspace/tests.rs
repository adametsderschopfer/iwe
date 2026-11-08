use super::*;

#[test]
fn editing_spaces_keeps_selection_valid_when_projects_are_removed() {
    let mut workspace = Workspace::default();
    workspace.create_space(
        "First".to_owned(),
        vec![PathBuf::from("a"), PathBuf::from("b")],
    );
    workspace.create_space("Other".to_owned(), vec![PathBuf::from("c")]);
    workspace.select_project(0, 1);
    assert!(workspace.update_space(
        0,
        "Renamed".to_owned(),
        vec![PathBuf::from("a"), PathBuf::from("a")]
    ));
    assert_eq!(workspace.spaces[0].projects.len(), 1);
    assert_eq!(workspace.directory(), Some(&PathBuf::from("a")));
    assert!(workspace.update_space(0, "Empty".to_owned(), Vec::new()));
    assert_eq!(workspace.directory(), Some(&PathBuf::from("c")));
    assert!(workspace.spaces[0].projects.is_empty());
    assert!(workspace.update_space(1, "Other".to_owned(), Vec::new()));
    assert!(workspace.directory().is_none());
}

#[test]
fn spaces_are_named_containers_and_projects_are_directories() {
    let mut workspace = Workspace::default();
    assert_eq!(workspace.create_space("  ".to_owned(), Vec::new()), None);
    assert_eq!(
        workspace.create_space(" Work ".to_owned(), Vec::new()),
        Some(0)
    );
    assert_eq!(workspace.spaces[0].name, "Work");
    assert!(workspace.spaces[0].projects.is_empty());
    assert!(workspace.directory().is_none());
    assert!(!workspace.select_project(0, 0));
    assert!(workspace.add_project(0, PathBuf::from("/first")));
    assert!(workspace.add_project(0, PathBuf::from("/second")));
    assert!(workspace.add_project(0, PathBuf::from("/first")));
    assert_eq!(workspace.spaces[0].projects.len(), 2);
    assert_eq!(workspace.directory(), Some(&PathBuf::from("/first")));
    assert!(workspace.select_directory(&PathBuf::from("/second")));
    assert_eq!(workspace.selected_project, Some(1));
    assert!(!workspace.add_project(99, PathBuf::from("/missing")));
    assert_eq!(workspace.spaces.len(), 1);
    assert_eq!(workspace.spaces[0].projects[1].name, "second");
    workspace.create_space("Personal".to_owned(), vec![PathBuf::from("/second")]);
    assert!(workspace.select_directory(&PathBuf::from("/second")));
    assert_eq!(workspace.selected_space, Some(1));
}

#[test]
fn empty_spaces_survive_roundtrip_and_keep_the_current_project() {
    let mut workspace = Workspace::default();
    workspace.create_space(
        "Work".to_owned(),
        vec![PathBuf::from("code"), PathBuf::from("code")],
    );
    workspace.create_space("Personal".to_owned(), Vec::new());
    let serialized = serde_json::to_value(&workspace).unwrap();
    assert!(serialized.get("spaces").is_some());
    assert!(serialized.get("projects").is_none());
    let mut restored: Workspace = serde_json::from_value(serialized).unwrap();
    restored.normalize();
    assert_eq!(restored.spaces.len(), 2);
    assert_eq!(restored.spaces[0].projects.len(), 1);
    assert!(restored.spaces[1].projects.is_empty());
    assert_eq!(restored.directory(), Some(&PathBuf::from("code")));
}

#[test]
fn legacy_groups_keep_names_directories_and_selection() {
    let mut workspace: Workspace = serde_json::from_str(r#"{"projects":[{"name":"empty","directories":[]},{"name":"Team","directories":["frontend","backend"]}],"selected_project":1,"selected_directory":1,"preferences":{"language":"es","code_size":18}}"#).unwrap();
    workspace.normalize();
    assert_eq!(workspace.spaces.len(), 2);
    assert_eq!(workspace.spaces[1].name, "Team");
    assert_eq!(
        workspace.spaces[1].projects[0].directory,
        PathBuf::from("frontend")
    );
    assert_eq!(workspace.spaces[1].projects[1].name, "backend");
    assert_eq!(workspace.directory(), Some(&PathBuf::from("backend")));
    assert_eq!(workspace.preferences.language.as_deref(), Some("es"));
    assert_eq!(workspace.preferences.code_size, 18);
}

#[test]
fn restored_state_recovers_invalid_selections_and_old_preferences() {
    let mut workspace: Workspace = serde_json::from_str(r#"{"projects":[{"name":"empty","directories":[]},{"name":"project","directories":["src"]}],"selected_project":99,"selected_directory":7,"preferences":{"code_size":100}}"#).unwrap();
    workspace.normalize();
    assert_eq!(workspace.spaces.len(), 2);
    assert_eq!(workspace.directory(), Some(&PathBuf::from("src")));
    assert_eq!(workspace.preferences.code_size, 22);
    assert!(workspace.preferences.show_git);
}

#[test]
fn legacy_panel_preferences_keep_only_one_panel_open() {
    let mut workspace: Workspace = serde_json::from_str(
        r#"{"spaces":[],"preferences":{"show_git":true,"show_pipelines":true}}"#,
    )
    .unwrap();
    workspace.normalize();
    assert!(workspace.preferences.show_git);
    assert!(!workspace.preferences.show_pipelines);
    workspace.preferences.show_git = false;
    workspace.preferences.show_pipelines = true;
    workspace.normalize();
    assert!(workspace.preferences.show_pipelines);
}
