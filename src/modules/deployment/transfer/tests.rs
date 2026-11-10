use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
    fail_read: bool,
}
impl Transport for Memory {
    fn list(&mut self, path: &str) -> Result<Vec<Entry>, Error> {
        Ok(self
            .files
            .iter()
            .filter(|(name, _)| {
                name.rsplit_once('/')
                    .is_some_and(|(parent, _)| parent == path.trim_end_matches('/'))
            })
            .map(|(path, bytes)| Entry {
                path: path.clone(),
                name: path.rsplit('/').next().unwrap().into(),
                directory: false,
                symlink: false,
                size: bytes.len() as u64,
            })
            .collect())
    }
    fn stat(&mut self, path: &str) -> Result<Option<Entry>, Error> {
        Ok(if let Some(bytes) = self.files.get(path) {
            Some(Entry {
                path: path.into(),
                name: path.into(),
                directory: false,
                symlink: false,
                size: bytes.len() as u64,
            })
        } else if path == "/"
            || self
                .files
                .keys()
                .any(|name| name.starts_with(&format!("{path}/")))
        {
            Some(Entry {
                path: path.into(),
                name: path.into(),
                directory: true,
                symlink: false,
                size: 0,
            })
        } else {
            None
        })
    }
    fn mkdir(&mut self, _: &str) -> Result<(), Error> {
        Ok(())
    }
    fn read(
        &mut self,
        path: &str,
        output: &mut dyn Write,
        progress: &Progress,
    ) -> Result<(), Error> {
        if self.fail_read {
            output.write_all(b"partial").unwrap();
            return Err(Error::Connection);
        }
        transport::copy(
            &mut self.files.get(path).ok_or(Error::Access)?.as_slice(),
            output,
            progress,
        )
    }
    fn write(
        &mut self,
        path: &str,
        input: &mut dyn Read,
        progress: &Progress,
    ) -> Result<(), Error> {
        let mut bytes = Vec::new();
        transport::copy(input, &mut bytes, progress)?;
        self.files.insert(path.into(), bytes);
        Ok(())
    }
}
fn profile(root: &Path) -> Profile {
    Profile {
        name: "Stage".into(),
        connection: "server".into(),
        project: root.into(),
        root: "/srv".into(),
        mappings: vec![Default::default()],
        ..Default::default()
    }
}

#[test]
fn compare_never_overwrites_local_file_and_upload_uses_disk() {
    let temp = tempfile::tempdir().unwrap();
    let local = temp.path().join("file.txt");
    fs::write(&local, "local").unwrap();
    let mut remote = Memory::default();
    remote
        .files
        .insert("/srv/file.txt".into(), b"server".to_vec());
    let profile = profile(temp.path());
    let output = run(
        &mut remote,
        &profile,
        &Operation::Compare(local.clone()),
        &Progress::default(),
    )
    .unwrap();
    let Output::Preview {
        text,
        previous: Some(previous),
        ..
    } = output
    else {
        panic!("comparison required")
    };
    assert_eq!(text.as_str(), "local");
    assert_eq!(previous.as_str(), "server");
    assert_eq!(fs::read_to_string(&local).unwrap(), "local");
    fs::write(&local, "latest").unwrap();
    run(
        &mut remote,
        &profile,
        &Operation::Upload(local),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(remote.files["/srv/file.txt"], b"latest");
}

#[test]
fn download_requires_confirmation_and_failed_download_preserves_file() {
    let temp = tempfile::tempdir().unwrap();
    let local = temp.path().join("file.txt");
    fs::write(&local, "local").unwrap();
    let profile = profile(temp.path());
    let mut remote = Memory::default();
    remote
        .files
        .insert("/srv/file.txt".into(), b"server".to_vec());
    assert!(matches!(
        run(
            &mut remote,
            &profile,
            &Operation::Download("/srv/file.txt".into(), false),
            &Progress::default()
        ),
        Err(Error::Overwrite)
    ));
    remote.fail_read = true;
    assert!(
        run(
            &mut remote,
            &profile,
            &Operation::Download("/srv/file.txt".into(), true),
            &Progress::default()
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&local).unwrap(), "local");
    remote.fail_read = false;
    run(
        &mut remote,
        &profile,
        &Operation::Download("/srv/file.txt".into(), true),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&local).unwrap(), "server");
}

#[test]
fn recursive_upload_skips_git_and_excluded_folders() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".git")).unwrap();
    fs::write(temp.path().join(".git/config"), "secret").unwrap();
    fs::write(temp.path().join("file.txt"), "file").unwrap();
    fs::write(temp.path().join("secret.txt"), "secret").unwrap();
    let mut profile = profile(temp.path());
    profile.excluded.push("secret.txt".into());
    let mut remote = Memory::default();
    run(
        &mut remote,
        &profile,
        &Operation::Upload(temp.path().to_owned()),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(remote.files.len(), 1);
    assert_eq!(remote.files["/srv/file.txt"], b"file");
}

#[cfg(unix)]
#[test]
fn local_symbolic_links_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(external.path(), temp.path().join("escape")).unwrap();
    assert_eq!(
        check_local(temp.path(), &temp.path().join("escape/file"), true),
        Err(Error::Access)
    );
}
