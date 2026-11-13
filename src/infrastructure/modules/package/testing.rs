//! Build module package archives for tests.

use crate::domain::modules::{Manifest, current_target};
use std::{fs::File, io::Write, path::Path};

pub(in crate::infrastructure::modules) fn test_manifest(
    id: &str,
    version: &str,
    entry: &str,
) -> Manifest {
    use std::collections::{BTreeMap, BTreeSet};
    let mut binaries = BTreeMap::new();
    binaries.insert(current_target().to_owned(), entry.to_owned());
    Manifest {
        id: id.to_owned(),
        name: crate::domain::modules::LocalizedText {
            en: format!("{id} module"),
            ru: None,
            es: None,
        },
        description: crate::domain::modules::LocalizedText {
            en: "A test module".to_owned(),
            ru: None,
            es: None,
        },
        version: semver::Version::parse(version).expect("valid version"),
        api: 1,
        min_app_version: semver::Version::new(1, 0, 0),
        entry: binaries,
        permissions: BTreeSet::new(),
        placement: crate::domain::modules::Placement::Bottom,
    }
}

/// Write a package archive with one manifest and one binary for this target.
pub(in crate::infrastructure::modules) fn build_package(
    archive: &Path,
    id: &str,
    version: &str,
    binary: &[u8],
) {
    build_package_with(archive, test_manifest(id, version, "module-bin"), binary)
}

/// Write a package archive from a custom manifest.
pub(in crate::infrastructure::modules) fn build_package_with(
    archive: &Path,
    manifest: Manifest,
    binary: &[u8],
) {
    let file = File::create(archive).expect("create archive");
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    append(
        &mut tar,
        "manifest.json",
        serde_json::to_string(&manifest)
            .expect("serialize manifest")
            .as_bytes(),
    );
    let entry = manifest
        .entry
        .get(current_target())
        .cloned()
        .unwrap_or_else(|| "module-bin".to_owned());
    append(
        &mut tar,
        &format!("binaries/{}/{entry}", current_target()),
        binary,
    );
    tar.into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip")
        .flush()
        .expect("flush archive");
}

fn append<W: Write>(tar: &mut tar::Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes)
        .expect("append entry");
}
