use super::*;
use crate::domain::auth::Secret;

async fn fixture(
    responses: Vec<(&'static str, Value)>,
) -> (Http, tokio::task::JoinHandle<Vec<String>>) {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http = Http::fixture(&format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut chunk = [0; 2048];
                let count = socket.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length = header
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .map(|value| value.parse::<usize>().unwrap())
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(bytes).unwrap());
            let body = body.to_string();
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (http, server)
}

fn remote(provider: Provider) -> Remote {
    Remote {
        host: if provider == Provider::GitLab {
            "gitlab.example.test:8443"
        } else {
            "github.com"
        }
        .into(),
        project: if provider == Provider::GitLab {
            "group/sub/app"
        } else {
            "group/app"
        }
        .into(),
        provider: Some(provider),
    }
}

fn run() -> Pipeline {
    Pipeline {
        id: 500,
        number: Some(7),
        title: "Build".into(),
        branch: "main".into(),
        commit: None,
        status: "running".into(),
        date: String::new(),
        url: "https://example.test/pipelines/500".into(),
    }
}

fn credential() -> Credential {
    Credential {
        access_token: Secret::new("fixture-token".into()),
        refresh_token: None,
        expires_at: None,
        client_id: None,
        username: "developer".into(),
    }
}

fn gitlab_job(id: u64, stage: &str, status: &str, capability: &str) -> Value {
    let mut value = json!({"id":format!("gid://gitlab/Ci::Build/{id}"), "name":format!("job-{id}"), "stage":{"name":stage}, "status":status, "webPath":format!("/group/sub/app/-/jobs/{id}")});
    value[capability] = true.into();
    value
}

fn gitlab_page(jobs: Vec<Value>, next: Option<&str>) -> Value {
    json!({"data":{"project":{"pipeline":{
        "id":"gid://gitlab/Ci::Pipeline/500", "cancelable":true,
        "stages":{"nodes":[{"name":"build"},{"name":"test"},{"name":"deploy"}],"pageInfo":{"hasNextPage":false}},
        "jobs":{"nodes":jobs,"pageInfo":{"hasNextPage":next.is_some(),"endCursor":next}}
    }}}})
}

#[tokio::test]
async fn gitlab_stage_status_respects_the_provider_result_for_permitted_job_failures() {
    let mut page = gitlab_page(vec![gitlab_job(20, "build", "FAILED", "retryable")], None);
    page["data"]["project"]["pipeline"]["stages"]["nodes"][0]["status"] = "success".into();
    page["data"]["project"]["pipeline"]["stages"]["nodes"][1]["status"] = "RUNNING".into();
    let (http, server) = fixture(vec![("200 OK", page)]).await;
    let details = load(
        &http,
        &remote(Provider::GitLab),
        &run(),
        Some(&credential()),
    )
    .await
    .unwrap();
    assert_eq!(details.stages[0].jobs[0].status, "failed");
    assert_eq!(details.stages[0].status(), Status::Passed);
    assert_eq!(details.stages[1].status(), Status::Running);
    assert_eq!(details.stages[2].status(), Status::Skipped);
    server.await.unwrap();
}

#[tokio::test]
async fn gitlab_keeps_provider_stage_order_and_loads_all_pages_with_current_capabilities() {
    let (http, server) = fixture(vec![
        (
            "200 OK",
            gitlab_page(
                vec![
                    gitlab_job(30, "deploy", "MANUAL", "playable"),
                    gitlab_job(10, "test", "RUNNING", "cancelable"),
                ],
                Some("page-2"),
            ),
        ),
        (
            "200 OK",
            gitlab_page(vec![gitlab_job(20, "build", "FAILED", "retryable")], None),
        ),
    ])
    .await;
    let details = load(
        &http,
        &remote(Provider::GitLab),
        &run(),
        Some(&credential()),
    )
    .await
    .unwrap();
    assert_eq!(
        details
            .stages
            .iter()
            .map(|stage| stage.name.as_deref().unwrap())
            .collect::<Vec<_>>(),
        ["build", "test", "deploy"]
    );
    for (index, id, action) in [
        (0, 20, JobAction::Retry),
        (1, 10, JobAction::Cancel),
        (2, 30, JobAction::Play),
    ] {
        let job = &details.stages[index].jobs[0];
        assert_eq!(job.id, id);
        assert_eq!(job.actions, [action]);
        assert!(
            job.url
                .starts_with("https://gitlab.example.test:8443/group/sub/app/-/jobs/")
        );
    }
    assert_eq!(details.stages[1].jobs[0].status, "running");
    assert!(details.cancelable);
    let requests = server.await.unwrap();
    for request in &requests {
        assert!(request.starts_with("POST /api/graphql "));
        assert!(request.contains("Bearer fixture-token"));
    }
    let body: Value = serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body["variables"],
        json!({"project":"group/sub/app", "iid":"7", "after":"page-2"})
    );
}

#[tokio::test]
async fn gitlab_resolves_missing_iid_and_allows_anonymous_reads() {
    let (http, server) = fixture(vec![
        ("200 OK", json!({"iid":7})),
        ("200 OK", gitlab_page(Vec::new(), None)),
    ])
    .await;
    let mut pipeline = run();
    pipeline.number = None;
    assert!(
        load(&http, &remote(Provider::GitLab), &pipeline, None)
            .await
            .is_ok()
    );
    let requests = server.await.unwrap();
    assert!(requests[0].starts_with("GET /api/v4/projects/group%2Fsub%2Fapp/pipelines/500 "));
    assert!(
        requests
            .iter()
            .all(|request| !request.to_ascii_lowercase().contains("authorization:"))
    );
}

#[tokio::test]
async fn gitlab_rejects_incomplete_stages_duplicate_cursors_and_wrong_pipeline() {
    let mut stages = gitlab_page(Vec::new(), None);
    stages["data"]["project"]["pipeline"]["stages"]["pageInfo"]["hasNextPage"] = true.into();
    let mut wrong = gitlab_page(Vec::new(), None);
    wrong["data"]["project"]["pipeline"]["id"] = "gid://gitlab/Ci::Pipeline/501".into();
    for (response, error) in [
        (stages, Failure::TooLarge),
        (wrong, Failure::Read),
        (
            json!({"errors":[{"message":"denied"}]}),
            Failure::Unavailable,
        ),
        (json!({"data":{"project":null}}), Failure::Access),
    ] {
        let (http, server) = fixture(vec![("200 OK", response)]).await;
        assert!(
            matches!(load(&http, &remote(Provider::GitLab), &run(), None).await, Err(value) if value == error)
        );
        server.await.unwrap();
    }
    let (http, server) = fixture(vec![
        (
            "200 OK",
            gitlab_page(
                vec![gitlab_job(1, "build", "RUNNING", "cancelable")],
                Some("same"),
            ),
        ),
        (
            "200 OK",
            gitlab_page(
                vec![gitlab_job(2, "build", "RUNNING", "cancelable")],
                Some("same"),
            ),
        ),
    ])
    .await;
    assert!(matches!(
        load(&http, &remote(Provider::GitLab), &run(), None).await,
        Err(Failure::Read)
    ));
    server.await.unwrap();
}

fn github_job(id: u64, status: &str, conclusion: Value) -> Value {
    json!({"id":id,"name":format!("job-{id}"),"status":status,"conclusion":conclusion,"html_url":format!("https://github.com/group/app/actions/runs/500/job/{id}")})
}

#[tokio::test]
async fn github_reads_all_latest_jobs_and_exposes_retry_without_individual_cancellation() {
    let first: Vec<_> = (1..=100)
        .map(|id| github_job(id, "completed", "success".into()))
        .collect();
    let (http, server) = fixture(vec![
        ("200 OK", json!({"total_count":103,"jobs":first})),
        ("200 OK", json!({"total_count":103,"jobs":[github_job(101,"in_progress",Value::Null), github_job(102,"completed","skipped".into()), github_job(103,"completed","failure".into())]})),
    ]).await;
    let details = load(&http, &remote(Provider::GitHub), &run(), None)
        .await
        .unwrap();
    assert!(details.stages[0].name.is_none());
    let jobs = &details.stages[0].jobs;
    assert_eq!(jobs.len(), 103);
    assert!(jobs[100].actions.is_empty());
    assert!(jobs[101].actions.is_empty());
    assert_eq!(jobs[102].actions, [JobAction::Retry]);
    assert!(details.cancelable);
    let requests = server.await.unwrap();
    assert!(requests[1].starts_with(
        "GET /repos/group/app/actions/runs/500/jobs?filter=latest&per_page=100&page=2 "
    ));
}

#[tokio::test]
async fn github_rejects_oversized_or_incomplete_job_lists() {
    for (response, error) in [
        (json!({"total_count":1001,"jobs":[]}), Failure::TooLarge),
        (json!({"total_count":1,"jobs":[]}), Failure::Read),
        (
            json!({"total_count":2,"jobs":[github_job(1,"completed","failure".into()),github_job(1,"completed","failure".into())]}),
            Failure::Read,
        ),
    ] {
        let (http, server) = fixture(vec![("200 OK", response)]).await;
        assert!(
            matches!(load(&http, &remote(Provider::GitHub), &run(), None).await, Err(value) if value == error)
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn job_actions_use_authenticated_posts_to_the_expected_provider_resource() {
    for (provider, action, path) in [
        (
            Provider::GitLab,
            Action::Job(42, JobAction::Play),
            "/api/v4/projects/group%2Fsub%2Fapp/jobs/42/play",
        ),
        (
            Provider::GitLab,
            Action::Job(42, JobAction::Retry),
            "/api/v4/projects/group%2Fsub%2Fapp/jobs/42/retry",
        ),
        (
            Provider::GitLab,
            Action::Job(42, JobAction::Cancel),
            "/api/v4/projects/group%2Fsub%2Fapp/jobs/42/cancel",
        ),
        (
            Provider::GitLab,
            Action::CancelPipeline,
            "/api/v4/projects/group%2Fsub%2Fapp/pipelines/500/cancel",
        ),
        (
            Provider::GitHub,
            Action::Job(42, JobAction::Retry),
            "/repos/group/app/actions/jobs/42/rerun",
        ),
        (
            Provider::GitHub,
            Action::CancelPipeline,
            "/repos/group/app/actions/runs/500/cancel",
        ),
    ] {
        let (http, server) = fixture(vec![("202 Accepted", json!({}))]).await;
        control(&http, &remote(provider), 500, action, &credential())
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with(&format!("POST {path} ")));
        assert!(requests[0].contains("Bearer fixture-token"));
        if provider == Provider::GitHub {
            assert!(
                requests[0]
                    .to_ascii_lowercase()
                    .contains("x-github-api-version: 2022-11-28")
            );
        }
    }
}

#[tokio::test]
async fn denied_or_conflicting_actions_surface_errors_without_resubmission() {
    for (status, error) in [
        ("401 Unauthorized", Failure::Access),
        ("403 Forbidden", Failure::PermissionDenied),
        ("409 Conflict", Failure::Unavailable),
        ("429 Too Many Requests", Failure::RateLimited),
    ] {
        let (http, server) = fixture(vec![(status, json!({}))]).await;
        assert_eq!(
            control(
                &http,
                &remote(Provider::GitLab),
                500,
                Action::Job(42, JobAction::Play),
                &credential()
            )
            .await,
            Err(error)
        );
        assert_eq!(server.await.unwrap().len(), 1);
    }
}
