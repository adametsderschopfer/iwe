use super::*;

#[test]
fn switching_project_cancels_requests_and_ignores_stale_results() {
    let mut state = State::default();
    state.sync_context(Some(0), Some(Path::new("/first")));
    let previous = state.progress.clone();
    let request = state.request;
    state.sync_context(Some(0), Some(Path::new("/second")));
    assert!(previous.cancel.is_cancelled());
    let _ = state.update(
        Event::Finished(request, Ok(Output::Listed("/srv".into(), vec![]))),
        &mut Connections::default(),
        &mut vec![],
        Some(Path::new("/second")),
    );
    assert!(state.tree.is_empty());
}

#[test]
fn selection_is_limited_to_profiles_of_the_active_project() {
    let state = State::default();
    let profiles = vec![
        Profile {
            id: "first".into(),
            project: "/one".into(),
            default: true,
            ..Default::default()
        },
        Profile {
            id: "second".into(),
            project: "/two".into(),
            default: true,
            ..Default::default()
        },
    ];
    assert_eq!(
        state
            .active_profile(&profiles, Some(Path::new("/two")))
            .unwrap()
            .id,
        "second"
    );
    assert!(
        state
            .active_profile(&profiles, Some(Path::new("/three")))
            .is_none()
    );
}

#[test]
fn failed_credentials_do_not_commit_connection_metadata() {
    let mut state = State::default();
    let mut settings = Connections::default();
    let mut profiles = vec![];
    state.credential_request = 5;
    state.saving = true;
    let _ = state.update(
        Event::ConnectionSaved(5, Connection::default(), Err(Error::Credentials)),
        &mut settings,
        &mut profiles,
        None,
    );
    assert!(settings.connections.is_empty());
    assert!(!state.saving);
    assert_eq!(state.error, Some(Error::Credentials));
}

#[test]
fn nested_key_dialog_preserves_connection_and_selects_saved_key() {
    let mut state = State::default();
    let mut settings = Connections::default();
    let mut profiles = vec![];
    let _ = state.update(Event::NewConnection, &mut settings, &mut profiles, None);
    let _ = state.update(
        Event::Input(Field::Host, "example.org".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::NewKey, &mut settings, &mut profiles, None);
    let _ = state.update(Event::AddKey, &mut settings, &mut profiles, None);
    assert!(state.key_dialog);
    assert_eq!(state.key_error, Some(Error::Configuration));
    assert!(settings.keys.is_empty());
    let _ = state.update(
        Event::Input(Field::KeyName, "Work".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let path = std::env::temp_dir().join("test-private-key");
    let _ = state.update(
        Event::Input(Field::KeyPath, path.to_string_lossy().into_owned()),
        &mut settings,
        &mut profiles,
        None,
    );
    let (_, effect) = state.update(Event::AddKey, &mut settings, &mut profiles, None);
    assert!(matches!(effect, Effect::Save));
    assert!(!state.key_dialog);
    assert!(state.key_error.is_none());
    assert_eq!(settings.keys.len(), 1);
    assert_eq!(settings.keys[0].path, path);
    let connection = state.connection.as_ref().unwrap();
    assert_eq!(connection.host, "example.org");
    assert_eq!(
        connection.authentication,
        Authentication::Key(settings.keys[0].id.clone())
    );
    assert!(settings.connections.is_empty());
}

#[test]
fn cancelled_key_dialog_ignores_late_picker_and_keeps_saved_keys_unchanged() {
    let mut state = State::default();
    let mut settings = Connections::default();
    let mut profiles = vec![];
    let _ = state.update(Event::NewKey, &mut settings, &mut profiles, None);
    let request = state.key_request;
    let _ = state.update(Event::CloseDialog, &mut settings, &mut profiles, None);
    let _ = state.update(Event::NewKey, &mut settings, &mut profiles, None);
    let _ = state.update(
        Event::KeyPicked(request, Some(std::env::temp_dir().join("old-key"))),
        &mut settings,
        &mut profiles,
        None,
    );
    assert!(state.key_path.is_empty());
    let _ = state.update(Event::CloseDialog, &mut settings, &mut profiles, None);
    assert!(!state.has_dialog());
    assert!(settings.keys.is_empty());
}

#[test]
fn closing_connection_discards_draft_and_clears_password() {
    let mut state = State::default();
    let mut settings = Connections::default();
    let mut profiles = vec![];
    let _ = state.update(Event::NewConnection, &mut settings, &mut profiles, None);
    let _ = state.update(
        Event::Password(Secret(zeroize::Zeroizing::new("secret".into()))),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::CloseDialog, &mut settings, &mut profiles, None);
    assert!(!state.has_dialog());
    assert!(state.secret.0.is_empty());
    assert!(!state.secret_changed);
    assert!(settings.connections.is_empty());
}

#[test]
fn selecting_another_connection_requires_discarding_changes() {
    let mut settings = Connections {
        connections: vec![
            Connection {
                id: "one".into(),
                name: "One".into(),
                ..Default::default()
            },
            Connection {
                id: "two".into(),
                name: "Two".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let mut state = State::default();
    let mut profiles = vec![];
    let _ = state.update(Event::OpenConnections, &mut settings, &mut profiles, None);
    let _ = state.update(
        Event::Input(Field::Name, "Changed".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::EditConnection("one".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    assert_eq!(state.connection.as_ref().unwrap().name, "Changed");
    let _ = state.update(
        Event::EditConnection("two".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    assert!(state.pending.is_some());
    assert_eq!(state.connection.as_ref().unwrap().id, "one");
    let _ = state.update(Event::CloseDialog, &mut settings, &mut profiles, None);
    assert!(state.pending.is_none());
    assert_eq!(state.connection.as_ref().unwrap().name, "Changed");
    let _ = state.update(
        Event::EditConnection("two".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::DiscardChanges, &mut settings, &mut profiles, None);
    assert_eq!(state.connection.as_ref().unwrap().id, "two");
    assert_eq!(settings.connections[0].name, "One");
}

#[test]
fn named_connection_creation_is_cancelable_and_preserves_protocol() {
    let mut state = State::default();
    let mut settings = Connections::default();
    let mut profiles = vec![];
    let _ = state.update(
        Event::BeginName(NameAction::NewConnection),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::ConfirmName, &mut settings, &mut profiles, None);
    assert!(state.naming.is_some());
    assert!(state.connection.is_none());
    let _ = state.update(
        Event::Input(Field::DraftName, "Staging".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::NewProtocol(Protocol::FtpsImplicit),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::ConfirmName, &mut settings, &mut profiles, None);
    let draft = state.connection.as_ref().unwrap();
    assert_eq!(draft.name, "Staging");
    assert_eq!(draft.protocol, Protocol::FtpsImplicit);
    assert_eq!(draft.port, 990);
    assert!(!state.has_dialog());
    assert!(settings.connections.is_empty());
}

#[test]
fn duplicating_profile_keeps_mappings_without_duplicating_default() {
    let mut state = State::default();
    let (_, mut settings, _, project) = transfer_context();
    let mut profiles = vec![Profile {
        id: "source".into(),
        project,
        connection: "connection".into(),
        root: "/srv".into(),
        name: "Development".into(),
        default: true,
        mappings: vec![Mapping {
            local: "web".into(),
            remote: "public".into(),
        }],
        ..Default::default()
    }];
    let _ = state.update(
        Event::EditProfile("source".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::BeginName(NameAction::DuplicateProfile),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::Input(Field::DraftName, "Production".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::ConfirmName, &mut settings, &mut profiles, None);
    let draft = state.profile.as_ref().unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "Development");
    assert_eq!(profiles[1], *draft);
    assert!(!state.profile_dirty(&profiles));
    assert_ne!(draft.id, "source");
    assert_eq!(draft.name, "Production");
    assert!(!draft.default);
    assert_eq!(draft.mappings, profiles[0].mappings);
    let _ = state.update(
        Event::ProfileTab(ProfileTab::Mappings),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::MappingRemote(0, "release".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::ProfileTab(ProfileTab::Exclusions),
        &mut settings,
        &mut profiles,
        None,
    );
    assert_eq!(
        state.profile.as_ref().unwrap().mappings[0].remote,
        "release"
    );
    assert_eq!(profiles[0].mappings[0].remote, "public");
}

#[test]
fn editing_shared_key_keeps_identity_and_does_not_change_connection_authentication() {
    let mut settings = Connections {
        keys: vec![PrivateKey {
            id: "key".into(),
            name: "Old".into(),
            path: std::env::temp_dir().join("old"),
        }],
        ..Default::default()
    };
    let mut state = State::default();
    let mut profiles = vec![];
    let _ = state.update(Event::NewConnection, &mut settings, &mut profiles, None);
    let _ = state.update(Event::OpenKeys, &mut settings, &mut profiles, None);
    let _ = state.update(
        Event::EditKey("key".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(
        Event::Input(Field::KeyName, "Renamed".into()),
        &mut settings,
        &mut profiles,
        None,
    );
    let _ = state.update(Event::AddKey, &mut settings, &mut profiles, None);
    assert_eq!(settings.keys.len(), 1);
    assert_eq!(settings.keys[0].id, "key");
    assert_eq!(settings.keys[0].name, "Renamed");
    assert_eq!(
        state.connection.as_ref().unwrap().authentication,
        Authentication::Password
    );
    assert!(state.keys_open);
    assert!(!state.key_dialog);
}

fn transfer_context() -> (State, Connections, Vec<Profile>, PathBuf) {
    let root = std::env::temp_dir().join("iwe-background-test");
    let profiles = vec![Profile {
        id: "profile".into(),
        name: "Test server".into(),
        project: root.clone(),
        connection: "connection".into(),
        root: "/srv".into(),
        mappings: vec![Mapping::default()],
        default: true,
        ..Default::default()
    }];
    let settings = Connections {
        connections: vec![Connection {
            id: "connection".into(),
            name: "Test connection".into(),
            host: "localhost".into(),
            port: 22,
            username: "test".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut state = State::default();
    state.sync_context(Some(0), Some(&root));
    (state, settings, profiles, root)
}

#[test]
fn transfers_keep_panel_visibility_and_report_activity_outside_it() {
    for open in [false, true] {
        let (mut state, mut settings, mut profiles, root) = transfer_context();
        state.open = open;
        let _ = state.update(
            Event::Run(Operation::Upload(root.join("file.txt"))),
            &mut settings,
            &mut profiles,
            Some(&root),
        );
        assert_eq!(state.open, open);
        assert!(state.busy && state.activity_visible);
        assert_eq!(state.operation_name, "Test server");
        let id = state.request;
        let _ = state.update(
            Event::Finished(id, Err(Error::Access)),
            &mut settings,
            &mut profiles,
            Some(&root),
        );
        assert!(state.activity_expanded);
        assert_eq!(state.open, open);
        let _ = state.update(Event::Retry, &mut settings, &mut profiles, Some(&root));
        assert!(state.busy);
        assert_eq!(state.open, open);
        let _ = state.update(Event::Cancel, &mut settings, &mut profiles, Some(&root));
        assert!(state.progress.cancel.is_cancelled());
    }
}

#[test]
fn queued_comparison_opens_immediately_and_rejects_duplicate_requests() {
    let (mut state, mut settings, mut profiles, root) = transfer_context();
    let _ = state.update(
        Event::Run(Operation::Upload(root.join("upload.txt"))),
        &mut settings,
        &mut profiles,
        Some(&root),
    );
    let path = root.join("compare.txt");
    for _ in 0..2 {
        let (_, effect) = state.update(
            Event::Run(Operation::Compare(path.clone())),
            &mut settings,
            &mut profiles,
            Some(&root),
        );
        assert!(
            matches!(effect, Effect::CompareRequested { remote, .. } if remote == "/srv/compare.txt")
        );
        assert_eq!(state.queued(), 1);
        assert_eq!(state.comparison_error("profile", &path), None);
    }
    let _ = state.update(Event::Cancel, &mut settings, &mut profiles, Some(&root));
    assert_eq!(
        state.comparison_error("profile", &path),
        Some(Error::Cancelled)
    );
    assert_eq!(state.queued(), 0);
}

#[test]
fn comparison_mapping_errors_open_settings_without_starting_network_work() {
    let (mut state, mut settings, mut profiles, root) = transfer_context();
    profiles[0].mappings.clear();
    let (_, effect) = state.update(
        Event::Run(Operation::Compare(root.join("file.txt"))),
        &mut settings,
        &mut profiles,
        Some(&root),
    );
    assert!(matches!(effect, Effect::Settings));
    assert!(!state.busy);
    assert!(!state.open);
}

#[test]
fn suspension_cancels_transfers_and_invalidates_credentials_and_file_pickers() {
    let mut state = State::default();
    state.request = 7;
    state.credential_request = 11;
    state.key_request = 13;
    state.open = true;
    state.busy = true;
    state.key_dialog = true;
    let progress = state.progress.clone();
    state.suspend();
    assert!(progress.cancel.is_cancelled());
    assert!(!state.open && !state.busy && !state.key_dialog);
    assert!(state.request > 7 && state.credential_request > 11 && state.key_request > 13);
    let mut settings = Connections::default();
    for event in [
        Event::Finished(7, Ok(Output::Listed("/obsolete".into(), vec![]))),
        Event::ConnectionSaved(11, Connection::default(), Ok(())),
        Event::KeyPicked(13, Some(PathBuf::from("/obsolete"))),
    ] {
        let _ = state.update(event, &mut settings, &mut vec![], None);
    }
    assert!(state.tree.is_empty());
    assert!(state.key_path.is_empty());
    assert!(settings.connections.is_empty());
}

#[test]
fn editor_save_keeps_its_origin_profile_and_reports_completion_or_failure() {
    use std::sync::Arc;
    let mut state = State::default();
    state.selected_profile = Some("production".into());
    let mut profiles = vec![Profile {
        id: "stage".into(),
        name: "Stage".into(),
        connection: "server".into(),
        project: "/project".into(),
        ..Default::default()
    }];
    let mut settings = Connections {
        connections: vec![Connection {
            id: "server".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let event = Event::SaveText {
        profile: "stage".into(),
        token: 9,
        path: "/file.txt".into(),
        expected: Arc::new("before".into()),
        text: Arc::new("after".into()),
    };
    let (_, effect) = state.update(
        event,
        &mut settings,
        &mut profiles,
        Some(Path::new("/project")),
    );
    assert!(matches!(effect, Effect::None));
    assert_eq!(state.current.as_ref().unwrap().profile.id, "stage");
    assert_eq!(state.operation_name, "Stage");
    assert!(!state.open);
    let request = state.request;
    let (_, effect) = state.update(
        Event::Finished(request, Err(Error::Changed)),
        &mut settings,
        &mut profiles,
        Some(Path::new("/project")),
    );
    assert!(matches!(
        effect,
        Effect::EditorSaved(9, Err(Error::Changed))
    ));
    assert!(!state.busy);
    let _ = state.update(
        Event::Retry,
        &mut settings,
        &mut profiles,
        Some(Path::new("/project")),
    );
    let (_, effect) = state.update(
        Event::Finished(state.request, Ok(Output::SavedText(9))),
        &mut settings,
        &mut profiles,
        Some(Path::new("/project")),
    );
    assert!(matches!(effect, Effect::EditorSaved(9, Ok(()))));
}

#[test]
fn duplicate_unsaved_profile_preserves_both_profiles_after_reopening() {
    let (mut state, mut connections, mut profiles, project) = transfer_context();
    state.profile = profiles.pop();
    for event in [
        Event::BeginName(NameAction::DuplicateProfile),
        Event::Input(Field::DraftName, "Production".into()),
        Event::ConfirmName,
    ] {
        let _ = state.update(event, &mut connections, &mut profiles, Some(&project));
    }
    assert_eq!(profiles.len(), 2);
    assert_ne!(profiles[0].id, profiles[1].id);
    assert_eq!(profiles[0].name, "Test server");
    assert!(profiles[0].default);
    assert!(!profiles[1].default);
    let restored: Vec<Profile> =
        serde_json::from_str(&serde_json::to_string(&profiles).unwrap()).unwrap();
    assert_eq!(profiles, restored);
    let _ = state.update(
        Event::EditProfile(profiles[0].id.clone()),
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.profile.as_ref().unwrap().name, "Test server");
}

#[test]
fn mapping_form_accepts_absolute_local_and_root_relative_server_paths() {
    let (mut state, mut connections, mut profiles, project) = transfer_context();
    state.profile = Some(profiles[0].clone());
    for event in [
        Event::Input(Field::Root, "/home/bitrix/ext_www".into()),
        Event::MappingLocal(0, project.to_string_lossy().into_owned()),
        Event::MappingRemote(0, "/kdmarket.dev.legacy-agency.ru".into()),
        Event::SaveProfile,
    ] {
        let _ = state.update(event, &mut connections, &mut profiles, Some(&project));
    }
    assert!(state.error.is_none());
    assert!(!state.profile_dirty(&profiles));
    assert!(profiles[0].mappings[0].local.as_os_str().is_empty());
    let remote = "/home/bitrix/ext_www/kdmarket.dev.legacy-agency.ru/index.php";
    assert_eq!(
        profiles[0].to_remote(&project.join("index.php")).unwrap(),
        remote
    );
    assert_eq!(
        profiles[0].to_local(remote).unwrap(),
        project.join("index.php")
    );
    for local in [project.join("../escape"), project.with_file_name("outside")] {
        let _ = state.update(
            Event::MappingLocal(0, local.to_string_lossy().into_owned()),
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        let _ = state.update(
            Event::SaveProfile,
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        assert_eq!(state.error, Some(Error::Mapping));
    }
    let _ = state.update(
        Event::MappingLocal(0, project.to_string_lossy().into_owned()),
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    for remote in ["/../escape", "/www/../../escape"] {
        let _ = state.update(
            Event::MappingRemote(0, remote.into()),
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        let _ = state.update(
            Event::SaveProfile,
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        assert_eq!(state.error, Some(Error::Mapping));
    }
    let _ = state.update(
        Event::MappingRemote(0, "/".into()),
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    let _ = state.update(
        Event::SaveProfile,
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    assert!(state.error.is_none());
    assert_eq!(
        profiles[0].to_remote(&project.join("index.php")).unwrap(),
        "/home/bitrix/ext_www/index.php"
    );
    let _ = state.update(
        Event::AddMapping,
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    let _ = state.update(
        Event::SaveProfile,
        &mut connections,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.error, Some(Error::Mapping));
}

#[test]
fn folder_dialog_selection_and_cancel_ignore_late_network_results() {
    for choose in [false, true] {
        let (mut state, mut connections, mut profiles, project) = transfer_context();
        state.profile = Some(profiles[0].clone());
        let _ = state.update(
            Event::BrowseRoot,
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        assert!(state.has_dialog());
        let request = state.request;
        let progress = state.progress.clone();
        let event = if choose {
            Event::UseRoot("/selected".into())
        } else {
            Event::CloseDialog
        };
        let _ = state.update(event, &mut connections, &mut profiles, Some(&project));
        assert!(!state.has_dialog());
        assert!(!state.busy);
        assert!(progress.cancel.is_cancelled());
        assert_eq!(
            state.profile.as_ref().unwrap().root,
            if choose { "/selected" } else { "/srv" }
        );
        let _ = state.update(
            Event::Finished(request, Ok(Output::Listed("/".into(), vec![]))),
            &mut connections,
            &mut profiles,
            Some(&project),
        );
        assert!(state.tree.is_empty());
    }
}

#[test]
fn browsing_server_does_not_change_default_and_remote_commands_keep_their_target() {
    for remote in [false, true] {
        let (mut state, mut settings, mut profiles, project) = transfer_context();
        let mut second = profiles[0].clone();
        second.id = "second".into();
        second.default = false;
        second.root = "/other".into();
        profiles.push(second);
        let (_, effect) = state.update(
            Event::SelectProfile("second".into()),
            &mut settings,
            &mut profiles,
            Some(&project),
        );
        assert!(matches!(effect, Effect::None));
        assert!(profiles[0].default);
        assert!(!profiles[1].default);
        let _ = state.update(
            Event::Finished(state.request, Ok(Output::Listed("/other".into(), vec![]))),
            &mut settings,
            &mut profiles,
            Some(&project),
        );
        let event = if remote {
            Event::RemoteCompare("/other/file.txt".into())
        } else {
            Event::Run(Operation::Compare(project.join("file.txt")))
        };
        let (_, effect) = state.update(event, &mut settings, &mut profiles, Some(&project));
        let expected = if remote { "second" } else { "profile" };
        assert_eq!(state.current.as_ref().unwrap().profile.id, expected);
        assert!(matches!(effect, Effect::CompareRequested { profile, .. } if profile == expected));
        assert!(!state.open);
    }
}

#[test]
fn local_commands_without_default_require_an_explicit_target_without_saving_it() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    profiles[0].default = false;
    let operation = Operation::Upload(project.join("file.txt"));
    let _ = state.update(
        Event::Run(operation.clone()),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(state.target.is_some());
    assert!(!state.busy);
    let _ = state.update(
        Event::CloseDialog,
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(state.target.is_none());
    assert!(!state.busy);
    let _ = state.update(
        Event::Run(operation),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    let (_, effect) = state.update(
        Event::ChooseTarget("profile".into()),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(matches!(effect, Effect::None));
    assert_eq!(state.current.as_ref().unwrap().profile.id, "profile");
    assert!(!profiles[0].default);
    assert!(state.target.is_none());
}

#[test]
fn refresh_invalidates_expanded_folder_cache_and_collapse_keeps_loaded_entries() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    state.tree.insert("/srv/sub".into(), vec![]);
    state.expanded.insert("/srv/sub".into());
    let _ = state.update(
        Event::CollapseAll,
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(state.expanded.is_empty());
    assert!(!state.tree.is_empty());
    let _ = state.update(Event::Refresh, &mut settings, &mut profiles, Some(&project));
    assert!(state.tree.is_empty());
    assert!(
        matches!(state.current.as_ref().unwrap().operation, Operation::List(ref path) if path == "/srv")
    );
}

#[test]
fn test_connection_does_not_require_mappings_and_result_survives_invalid_profile_save() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    state.profile = Some(profiles[0].clone());
    state.profile.as_mut().unwrap().mappings.clear();
    let _ = state.update(
        Event::TestProfile,
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(state.busy);
    let _ = state.update(
        Event::Finished(state.request, Ok(Output::Connected)),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.test_result, Some(Ok(())));
    let _ = state.update(
        Event::SaveProfile,
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.error, Some(Error::Mapping));
    assert_eq!(state.test_result, Some(Ok(())));
}

#[test]
fn mapping_folder_browser_uses_profile_root_and_stores_relative_selection() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    state.profile = Some(profiles[0].clone());
    let _ = state.update(
        Event::BrowseMapping(0),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.browsing_root(), "/srv");
    assert_eq!(state.current.as_ref().unwrap().profile.root, "/srv");
    let request = state.request;
    let _ = state.update(
        Event::UseRoot("/srv/public".into()),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.profile.as_ref().unwrap().mappings[0].remote, "public");
    assert_eq!(state.profile.as_ref().unwrap().root, "/srv");
    assert!(!state.browsing);
    let _ = state.update(
        Event::Finished(request, Ok(Output::Listed("/srv".into(), vec![]))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(state.tree.is_empty());
}

#[test]
fn local_folder_picker_rejects_outside_project_and_ignores_obsolete_drafts() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    let snapshot = profiles[0].clone();
    state.profile = Some(snapshot.clone());
    let _ = state.update(
        Event::LocalMappingPicked(snapshot.clone(), 0, Some(project.join("public"))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(
        state.profile.as_ref().unwrap().mappings[0].local,
        project.join("public")
    );
    let _ = state.update(
        Event::LocalMappingPicked(snapshot, 0, Some(project.join("obsolete"))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(
        state.profile.as_ref().unwrap().mappings[0].local,
        project.join("public")
    );
    let current = state.profile.clone().unwrap();
    let _ = state.update(
        Event::LocalMappingPicked(current, 0, Some(project.with_file_name("outside"))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert_eq!(state.error, Some(Error::Mapping));
    assert_eq!(
        state.profile.as_ref().unwrap().mappings[0].local,
        project.join("public")
    );
}

#[test]
fn upload_without_mapping_opens_settings_before_connecting() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    profiles[0].mappings.clear();
    let (_, effect) = state.update(
        Event::Run(Operation::Upload(project.join("file.txt"))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    assert!(matches!(effect, Effect::Settings));
    assert!(!state.busy);
    assert_eq!(state.error, Some(Error::Mapping));
}

#[test]
fn cancelling_host_confirmation_replaces_the_test_result_without_retrying() {
    let (mut state, mut settings, mut profiles, project) = transfer_context();
    state.profile = Some(profiles[0].clone());
    let _ = state.update(
        Event::TestProfile,
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    let _ = state.update(
        Event::Finished(state.request, Err(Error::HostUnknown("SHA256:test".into()))),
        &mut settings,
        &mut profiles,
        Some(&project),
    );
    let _ = state.update(Event::Cancel, &mut settings, &mut profiles, Some(&project));
    assert_eq!(state.test_result, Some(Err(Error::Cancelled)));
    assert!(state.failed.is_none());
    assert!(!state.busy);
}
