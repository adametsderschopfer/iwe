//! Verify registry persistence round trips through temporary directories.

use super::{load_at, save_at};

#[test]
fn missing_registry_loads_empty() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let registry = load_at(&directory.path().join("modules.json")).expect("load missing registry");
    assert!(registry.modules.is_empty());
}

#[test]
fn registry_round_trips_through_disk() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("modules.json");
    let mut registry = crate::domain::modules::ModuleRegistry::default();
    registry
        .modules
        .push(crate::domain::modules::InstalledModule {
            manifest: manifest("sample"),
            installed_at: 7,
        });
    save_at(&path, &registry).expect("save registry");
    let loaded = load_at(&path).expect("load registry");
    assert_eq!(loaded, registry);
}

#[test]
fn corrupted_registry_reports_storage_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("modules.json");
    std::fs::write(&path, b"{").expect("write registry");
    assert!(load_at(&path).is_err());
}

fn manifest(id: &str) -> crate::domain::modules::Manifest {
    use std::collections::{BTreeMap, BTreeSet};
    let mut entry = BTreeMap::new();
    entry.insert(
        crate::domain::modules::current_target().to_owned(),
        "module-bin".to_owned(),
    );
    crate::domain::modules::Manifest {
        id: id.to_owned(),
        name: crate::domain::modules::LocalizedText {
            en: "Sample".to_owned(),
            ru: None,
            es: None,
        },
        description: crate::domain::modules::LocalizedText {
            en: "Sample module".to_owned(),
            ru: None,
            es: None,
        },
        version: semver::Version::new(1, 0, 0),
        api: 1,
        min_app_version: semver::Version::new(1, 0, 0),
        entry,
        permissions: BTreeSet::new(),
        placement: crate::domain::modules::Placement::Bottom,
    }
}
