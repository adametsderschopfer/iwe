use super::*;

#[test]
fn github_conclusions_override_completed_status() {
    let bytes = br#"{"workflow_runs":[{"id":42,"display_title":"Build","head_branch":"main","status":"completed","conclusion":"failure","created_at":"2026-11-08T10:00:00Z","html_url":"https://github.com/team/app/actions/runs/42"}]}"#;
    let runs = parse(bytes, Provider::GitHub).unwrap();
    assert_eq!(runs[0].status, "failure");
    assert_eq!(runs[0].branch, "main");
    assert_eq!(runs[0].id, 42);
}

#[test]
fn gitlab_pipeline_lists_and_empty_responses_are_supported() {
    let runs = parse(br#"[{"id":17,"status":"running","ref":"task/work","created_at":"2026-11-08T10:00:00Z","web_url":"https://gitlab.com/group/app/-/pipelines/17"}]"#, Provider::GitLab).unwrap();
    assert_eq!(runs[0].status, "running");
    assert_eq!(runs[0].branch, "task/work");
    assert!(parse(b"[]", Provider::GitLab).unwrap().is_empty());
    assert!(parse(br#"{"message":"401 Unauthorized"}"#, Provider::GitLab).is_err());
    assert!(parse(b"[{}]", Provider::GitHub).is_err());
}

#[tokio::test]
async fn external_links_reject_executable_schemes() {
    for url in ["file:///tmp/script", "javascript:alert(1)", "not a url"] {
        assert_eq!(open_url(url.to_owned()).await, Err(Failure::Read));
    }
}

#[test]
fn nested_gitlab_projects_use_one_encoded_path_segment() {
    let account = Account {
        provider: Provider::GitLab,
        host: "gitlab.example.com:8443".to_owned(),
    };
    assert_eq!(
        endpoint(&account, "group/subgroup/repository").unwrap(),
        "https://gitlab.example.com:8443/api/v4/projects/group%2Fsubgroup%2Frepository/pipelines?per_page=30"
    );
    let github = Account {
        provider: Provider::GitHub,
        host: "github.com".to_owned(),
    };
    assert_eq!(
        endpoint(&github, "team/app").unwrap(),
        "https://api.github.com/repos/team/app/actions/runs?per_page=30"
    );
    assert!(endpoint(&github, "team/app/more").is_err());
}

#[test]
fn providers_supply_full_commit_ids_for_history_badges() {
    let sha = "1234567890123456789012345678901234567890";
    for (provider, json) in [
        (
            Provider::GitHub,
            format!(
                r#"{{"workflow_runs":[{{"id":1,"head_sha":"{sha}","status":"in_progress","conclusion":null}}]}}"#
            ),
        ),
        (
            Provider::GitLab,
            format!(r#"[{{"id":1,"sha":"{sha}","status":"running"}}]"#),
        ),
    ] {
        let runs = parse(json.as_bytes(), provider).unwrap();
        assert_eq!(runs[0].commit.as_deref(), Some(sha));
        assert!(runs[0].is_active());
    }
    let runs = parse(
        br#"[{"id":1,"sha":"1234567","status":"running"},{"id":2,"status":"success"}]"#,
        Provider::GitLab,
    )
    .unwrap();
    assert!(runs.iter().all(|run| run.commit.is_none()));
    assert!(!runs[1].is_active());
}
