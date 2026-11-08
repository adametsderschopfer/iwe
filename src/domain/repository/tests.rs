use super::*;

#[test]
fn remote_detection_supports_ssh_https_and_nested_groups() {
    for (value, host, project, provider) in [
        (
            "git@github.com:team/app.git",
            "github.com",
            "team/app",
            Some(Provider::GitHub),
        ),
        (
            "https://gitlab.com/team/group/app.git",
            "gitlab.com",
            "team/group/app",
            Some(Provider::GitLab),
        ),
        (
            "ssh://git@gitlab.example.com:2222/team/app.git",
            "gitlab.example.com",
            "team/app",
            Some(Provider::GitLab),
        ),
        (
            "https://user:secret@github.example.com/team/app.git",
            "github.example.com",
            "team/app",
            Some(Provider::GitHub),
        ),
        (
            "https://code.example.com:8443/team/app.git",
            "code.example.com:8443",
            "team/app",
            None,
        ),
    ] {
        let remote = Remote::parse(value).unwrap();
        assert_eq!(
            (
                remote.host.as_str(),
                remote.project.as_str(),
                remote.provider
            ),
            (host, project, provider)
        );
        assert!(!format!("{remote:?}").contains("secret"));
    }
}

#[test]
fn local_paths_and_unsafe_hosts_do_not_become_providers() {
    for value in [
        "/tmp/repo",
        "file:///tmp/repo",
        "C:\\work\\repo",
        "https://github.com/app",
        "git@bad;host:team/repo",
        "invalid",
    ] {
        assert!(Remote::parse(value).is_none(), "{value}");
    }
}

#[test]
fn graph_tracks_merges_and_closes_parent_lanes() {
    let make = |id: &str, parents: &[&str]| Commit {
        id: id.to_owned(),
        parents: parents.iter().map(|value| (*value).to_owned()).collect(),
        subject: String::new(),
        author: String::new(),
        date: String::new(),
        refs: String::new(),
        graph: GraphRow::default(),
    };
    let mut commits = vec![
        make("merge", &["left", "right"]),
        make("left", &["root"]),
        make("right", &["root"]),
        make("root", &[]),
    ];
    assign_graph(&mut commits, Vec::new());
    assert!(commits[0].graph.incoming.is_empty());
    assert_eq!(commits[0].graph.outgoing, [(0, 0), (0, 1)]);
    assert_eq!(commits[2].graph.lane, 1);
    assert!(commits[2].graph.outgoing.contains(&(1, 0)));
    assert!(commits[3].graph.outgoing.is_empty());
    let mut paged = commits.clone();
    let mut lanes = Vec::new();
    for page in paged.chunks_mut(2) {
        lanes = assign_graph(page, lanes);
    }
    for (complete, page) in commits.iter().zip(paged) {
        assert_eq!(complete.graph.lane, page.graph.lane);
        assert_eq!(complete.graph.incoming, page.graph.incoming);
        assert_eq!(complete.graph.outgoing, page.graph.outgoing);
    }
    assert!(lanes.is_empty());
}
