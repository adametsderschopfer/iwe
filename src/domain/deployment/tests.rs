use super::*;

fn profile() -> Profile {
    Profile {
        name: "Stage".into(),
        connection: "server".into(),
        project: std::env::temp_dir().join("project"),
        root: "/srv/site".into(),
        mappings: vec![Mapping::default()],
        ..Profile::default()
    }
}

#[test]
fn maps_both_directions_and_rejects_escape() {
    let profile = profile();
    let local = profile.project.join("src/main.php");
    assert_eq!(
        profile.to_remote(&local),
        Ok("/srv/site/src/main.php".into())
    );
    assert_eq!(profile.to_local("/srv/site/src/main.php"), Ok(local));
    assert_eq!(
        profile.to_local("/srv/site-other/main.php"),
        Err(Error::Mapping)
    );
    assert_eq!(profile.to_local("/srv/site/../secret"), Err(Error::Mapping));
    assert_eq!(
        profile.to_remote(&profile.project.join("../secret")),
        Err(Error::Mapping)
    );
}

#[test]
fn rejects_overlapping_mappings_in_both_directions() {
    let mut profile = profile();
    profile.mappings.push(Mapping {
        local: "src".into(),
        remote: "other".into(),
    });
    assert_eq!(profile.validate(), Err(Error::Mapping));
    profile.mappings = vec![
        Mapping {
            local: "src".into(),
            remote: "app".into(),
        },
        Mapping {
            local: "static".into(),
            remote: "app/assets".into(),
        },
    ];
    assert_eq!(profile.validate(), Err(Error::Mapping));
    profile.mappings[1].remote = "public".into();
    assert_eq!(profile.validate(), Ok(()));
}

#[test]
fn exclusions_are_bidirectional_and_git_is_always_excluded() {
    let mut profile = profile();
    profile.excluded.push("secrets".into());
    assert_eq!(
        profile.to_remote(&profile.project.join("secrets/password")),
        Err(Error::Excluded)
    );
    assert_eq!(
        profile.to_local("/srv/site/secrets/password"),
        Err(Error::Excluded)
    );
    assert_eq!(
        profile.to_local("/srv/site/.git/config"),
        Err(Error::Excluded)
    );
    assert!(profile.to_local("/srv/site/secrets.txt").is_ok());
}

#[test]
fn legacy_workspace_loads_without_deployment_and_roundtrips() {
    let mut workspace: crate::domain::workspace::Workspace =
        serde_json::from_str(r#"{"spaces":[{"name":"Work","projects":[]}]}"#).unwrap();
    assert!(workspace.deployment.connections.is_empty());
    assert!(workspace.spaces[0].deployments.is_empty());
    workspace.spaces[0].deployments.push(profile());
    workspace.deployment.connections.push(Connection {
        id: "server".into(),
        name: "Stage".into(),
        host: "localhost".into(),
        port: 22,
        username: "user".into(),
        ..Connection::default()
    });
    let serialized = serde_json::to_string(&workspace).unwrap();
    let restored: crate::domain::workspace::Workspace = serde_json::from_str(&serialized).unwrap();
    assert_eq!(restored.spaces[0].deployments[0].name, "Stage");
    assert_eq!(restored.deployment.connections[0].id, "server");
}

#[test]
fn rejects_control_characters_in_paths_and_connections() {
    assert_eq!(remote_path("/srv/file\r\nDELE x"), Err(Error::Mapping));
    assert_eq!(remote_path("/srv/../../x"), Err(Error::Mapping));
    let connection = Connection {
        name: "test".into(),
        host: "localhost".into(),
        port: 21,
        username: "user\r\nPASS x".into(),
        ..Default::default()
    };
    assert_eq!(connection.validate(), Err(Error::Configuration));
}
