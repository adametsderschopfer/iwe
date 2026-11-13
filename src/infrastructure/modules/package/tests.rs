//! Verify package extraction, manifest limits, and path rejection.

use super::{Error, extract, manifest, testing};
use std::{fs::File, io::Write};

#[test]
fn reads_manifest_without_unpacking() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let archive = directory.path().join("sample.iwe-module");
    testing::build_package(&archive, "sample", "1.2.3", b"binary");
    let parsed = manifest(&archive).expect("read manifest");
    assert_eq!(parsed.id, "sample");
    assert_eq!(parsed.version.to_string(), "1.2.3");
}

#[test]
fn extracts_entry_for_current_target() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let archive = directory.path().join("sample.iwe-module");
    testing::build_package(&archive, "sample", "1.0.0", b"binary");
    let staging = tempfile::tempdir().expect("staging directory");
    let package = extract(&archive, staging.path()).expect("extract package");
    assert_eq!(package.manifest.id, "sample");
    assert!(staging.path().join(&package.entry).is_file());
}

#[test]
fn rejects_missing_archive_and_unsupported_target() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let missing = directory.path().join("missing.iwe-module");
    assert_eq!(manifest(&missing), Err(Error::Invalid));

    let mut only_other = testing::test_manifest("sample", "1.0.0", "module-bin");
    only_other
        .entry
        .remove(crate::domain::modules::current_target());
    only_other
        .entry
        .insert("other-platform".to_owned(), "module-bin".to_owned());
    let archive = directory.path().join("other.iwe-module");
    testing::build_package_with(&archive, only_other, b"binary");
    assert_eq!(manifest(&archive), Err(Error::UnsupportedTarget));
}

#[test]
fn rejects_declared_entry_missing_from_the_archive() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let archive = directory.path().join("ghost.iwe-module");
    let file = File::create(&archive).expect("create archive");
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let manifest = serde_json::to_string(&testing::test_manifest("sample", "1.0.0", "ghost"))
        .expect("serialize manifest");
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "manifest.json", manifest.as_bytes())
        .expect("append manifest");
    tar.into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip")
        .flush()
        .expect("flush archive");

    let staging = tempfile::tempdir().expect("staging directory");
    assert_eq!(extract(&archive, staging.path()), Err(Error::Invalid));
}
