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
