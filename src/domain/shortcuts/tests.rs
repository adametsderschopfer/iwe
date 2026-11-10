use super::*;

#[test]
fn assignments_reject_effective_defaults_and_keep_previous_binding() {
    let mut shortcuts = Shortcuts::default();
    let original = shortcuts.binding(Command::GoToLine);
    assert_eq!(
        shortcuts.assign(Command::GoToLine, shortcuts.binding(Command::Copy)),
        Err(Rejection::Conflict)
    );
    assert_eq!(shortcuts.binding(Command::GoToLine), original);
    let mut custom = original;
    custom.key = "K".into();
    shortcuts.assign(Command::GoToLine, custom.clone()).unwrap();
    assert_eq!(
        shortcuts.assign(Command::CloseTab, custom.clone()),
        Err(Rejection::Conflict)
    );
    assert_eq!(shortcuts.assign(Command::GoToLine, custom), Ok(()));
}

#[test]
fn assignments_survive_serialization_and_allow_distinct_modifiers() {
    let mut shortcuts = Shortcuts::default();
    let mut custom = Command::Copy.default_binding();
    custom.alt = true;
    shortcuts.assign(Command::GoToLine, custom.clone()).unwrap();
    let mut restored: Shortcuts =
        serde_json::from_str(&serde_json::to_string(&shortcuts).unwrap()).unwrap();
    restored.normalize();
    assert_eq!(restored.binding(Command::GoToLine), custom);
    assert_eq!(
        restored.binding(Command::Copy),
        Command::Copy.default_binding()
    );
}

#[test]
fn invalid_bindings_and_native_menu_conflicts_are_rejected() {
    let mut shortcuts = Shortcuts::default();
    for key in ["a", "Escape", "F0", "F13", "F01", "Control", ""] {
        let mut binding = Command::Copy.default_binding();
        binding.key = key.into();
        assert_eq!(
            shortcuts.assign(Command::Copy, binding),
            Err(Rejection::Invalid)
        );
    }
    let mut binding = Command::Copy.default_binding();
    binding.key = "Comma".into();
    assert_eq!(
        shortcuts.assign(Command::Copy, binding),
        Err(Rejection::Conflict)
    );
    let mut binding = Command::Copy.default_binding();
    binding.control = false;
    binding.super_key = false;
    assert_eq!(
        shortcuts.assign(Command::Copy, binding),
        Err(Rejection::Invalid)
    );
    if cfg!(target_os = "macos") {
        let mut binding = Command::Copy.default_binding();
        binding.key = "Q".into();
        assert_eq!(
            shortcuts.assign(Command::Copy, binding),
            Err(Rejection::Conflict)
        );
    }
}

#[test]
fn loading_conflicts_restores_unique_effective_bindings() {
    let mut shortcuts = Shortcuts(BTreeMap::from([
        (Command::GoToLine, Command::Copy.default_binding()),
        (Command::CloseTab, Command::GoToLine.default_binding()),
    ]));
    shortcuts.normalize();
    for command in Command::ALL {
        for other in Command::ALL {
            if command != other {
                assert_ne!(shortcuts.binding(command), shortcuts.binding(other));
            }
        }
    }
    let workspace: crate::domain::workspace::Workspace = serde_json::from_str("{}").unwrap();
    assert_eq!(
        workspace.preferences.shortcuts.binding(Command::Copy),
        Command::Copy.default_binding()
    );
}

#[test]
fn deployment_defaults_use_shift_and_preserve_custom_assignments() {
    let mut shortcuts = Shortcuts::default();
    for (command, key) in [
        (Command::DeploymentCompare, "C"),
        (Command::DeploymentUpload, "A"),
    ] {
        let binding = shortcuts.binding(command);
        assert_eq!(binding.key, key);
        assert!(binding.shift);
        assert!(!binding.alt);
        assert_eq!(binding.super_key, cfg!(target_os = "macos"));
        assert_eq!(binding.control, !cfg!(target_os = "macos"));
    }
    let mut custom = Command::DeploymentUpload.default_binding();
    custom.key = "U".into();
    shortcuts
        .assign(Command::DeploymentUpload, custom.clone())
        .unwrap();
    let mut restored: Shortcuts =
        serde_json::from_str(&serde_json::to_string(&shortcuts).unwrap()).unwrap();
    restored.normalize();
    assert_eq!(restored.binding(Command::DeploymentUpload), custom);
}
