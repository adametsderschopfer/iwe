//! Verify installation, updates, and removal through temporary directories.

use super::{install_into, uninstall_from};
use crate::domain::modules::ModuleRegistry;
use crate::infrastructure::modules::package::testing;
use std::path::Path;

fn context(name: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect(name);
    let root = directory.path().join("modules");
    let registry = directory.path().join("modules.json");
    (directory, root, registry)
}

#[test]
fn installs_package_and_persists_registry() {
    let (_guard, root, registry_file) = context("install");
    let packages = tempfile::tempdir().expect("packages");
    let archive = packages.path().join("sample.iwe-module");
    testing::build_package(&archive, "sample", "1.0.0", b"binary");
    let (updated, installed_id) =
        install_into(&archive, &ModuleRegistry::default(), &root, &registry_file)
            .expect("install package");
    assert_eq!(installed_id, "sample");
    assert_eq!(updated.modules.len(), 1);
    assert!(
        root.join("sample")
            .join("1.0.0")
            .join("binaries")
            .join(crate::domain::modules::current_target())
            .join("module-bin")
            .is_file()
    );
    assert!(registry_file.is_file());
}

#[test]
fn update_replaces_previous_version() {
    let (_guard, root, registry_file) = context("update");
    let packages = tempfile::tempdir().expect("packages");
    let first = packages.path().join("first.iwe-module");
    testing::build_package(&first, "sample", "1.0.0", b"one");
    let second = packages.path().join("second.iwe-module");
    testing::build_package(&second, "sample", "1.1.0", b"two");

    let (mut registry, _) =
        install_into(&first, &ModuleRegistry::default(), &root, &registry_file).expect("first");
    assert!(root.join("sample").join("1.0.0").is_dir());
    (registry, _) = install_into(&second, &registry, &root, &registry_file).expect("second");
    assert!(root.join("sample").join("1.1.0").is_dir());
    assert!(!root.join("sample").join("1.0.0").exists());
    assert_eq!(registry.modules.len(), 1);
    assert_eq!(
        registry.get("sample").unwrap().manifest.version.to_string(),
        "1.1.0"
    );
}

#[test]
fn rejects_same_or_older_version() {
    let (_guard, root, registry_file) = context("downgrade");
    let packages = tempfile::tempdir().expect("packages");
    let archive = packages.path().join("sample.iwe-module");
    testing::build_package(&archive, "sample", "2.0.0", b"binary");
    let (registry, _) =
        install_into(&archive, &ModuleRegistry::default(), &root, &registry_file).expect("first");
    let error = install_into(&archive, &registry, &root, &registry_file);
    assert_eq!(
        error,
        Err(crate::infrastructure::modules::Error::OlderVersion)
    );
}

#[test]
fn uninstall_removes_files_and_record() {
    let (_guard, root, registry_file) = context("uninstall");
    let packages = tempfile::tempdir().expect("packages");
    let archive = packages.path().join("sample.iwe-module");
    testing::build_package(&archive, "sample", "1.0.0", b"binary");
    let (registry, _) =
        install_into(&archive, &ModuleRegistry::default(), &root, &registry_file).expect("install");
    let registry = uninstall_from("sample", &registry, &root, &registry_file).expect("uninstall");
    assert!(registry.modules.is_empty());
    assert!(!root.join("sample").exists());
    assert!(Path::new(&registry_file).is_file());
}
