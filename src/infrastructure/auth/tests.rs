use super::*;
use std::{collections::BTreeMap, sync::Mutex as StdMutex};

#[derive(Default)]
struct MemoryStore(StdMutex<BTreeMap<String, Credential>>);

impl Store for MemoryStore {
    fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
        Ok(self.0.lock().unwrap().get(&account.storage_key()).cloned())
    }
    fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
        self.0
            .lock()
            .unwrap()
            .insert(account.storage_key(), credential.clone());
        Ok(())
    }
    fn delete(&self, account: &Account) -> Result<(), Failure> {
        self.0.lock().unwrap().remove(&account.storage_key());
        Ok(())
    }
}

fn service() -> Service {
    Service {
        http: Ok(Http::local()),
        store: Arc::new(MemoryStore::default()),
        gate: Arc::new(Mutex::new(())),
    }
}
fn account() -> Account {
    Account {
        provider: Provider::GitLab,
        host: "gitlab.example.test".to_owned(),
    }
}
fn credential() -> Credential {
    serde_json::from_str::<TokenResponse>(r#"{"access_token":"test-access","refresh_token":"test-refresh","expires_in":7200,"token_type":"Bearer"}"#).unwrap().credential("public-id").unwrap()
}

#[test]
fn token_responses_preserve_refresh_and_expiration_without_exposing_secrets() {
    let credential = credential();
    assert_eq!(
        credential.refresh_token.as_ref().unwrap().expose(),
        "test-refresh"
    );
    assert!(credential.expires_at.unwrap() >= now() + 7199);
    assert!(!format!("{credential:?}").contains("test-refresh"));
    for (json, error) in [
        (r#"{"error":"invalid_grant"}"#, Failure::AuthExpired),
        (r#"{"error":"access_denied"}"#, Failure::AuthDenied),
        (
            r#"{"error":"device_flow_disabled"}"#,
            Failure::AuthNotConfigured,
        ),
        (r#"{"access_token":""}"#, Failure::Read),
    ] {
        assert!(
            matches!(serde_json::from_str::<TokenResponse>(json).unwrap().credential("id"), Err(value) if value == error)
        );
    }
}

#[tokio::test]
async fn disconnect_removes_only_the_selected_server() {
    let service = service();
    let account = account();
    let other = Account {
        host: "another.example.test".to_owned(),
        ..account.clone()
    };
    service.write(account.clone(), credential()).await.unwrap();
    service.write(other.clone(), credential()).await.unwrap();
    service.disconnect(account.clone()).await.unwrap();
    assert!(service.read(account).await.unwrap().is_none());
    assert!(service.read(other).await.unwrap().is_some());
}

#[tokio::test]
async fn cancelled_and_invalid_token_attempts_never_reach_the_network_or_store() {
    let service = service();
    let account = account();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        service
            .connect_token(
                account.clone(),
                Secret::new("test-only-token".to_owned()),
                cancel
            )
            .await,
        Err(Failure::AuthCancelled)
    );
    for value in ["", "token\nheader", "two tokens"] {
        assert_eq!(
            service
                .connect_token(
                    account.clone(),
                    Secret::new(value.to_owned()),
                    CancellationToken::new()
                )
                .await,
            Err(Failure::InvalidToken)
        );
    }
    assert!(service.read(account).await.unwrap().is_none());
}

async fn fixture(
    responses: Vec<(&'static str, &'static str)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
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
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (origin, server)
}

#[tokio::test]
async fn self_hosted_token_is_validated_before_storage() {
    let (origin, server) = fixture(vec![
        ("401 Unauthorized", "{}"),
        ("200 OK", r#"{"username":"developer"}"#),
    ])
    .await;
    let service = Service {
        http: Ok(Http::fixture(&origin)),
        ..service()
    };
    let account = account();
    let connect = || {
        service.connect_token(
            account.clone(),
            Secret::new("test-pat".to_owned()),
            CancellationToken::new(),
        )
    };
    assert_eq!(connect().await, Err(Failure::InvalidToken));
    assert!(service.read(account.clone()).await.unwrap().is_none());
    assert_eq!(connect().await.unwrap(), "developer");
    assert_eq!(
        service.read(account).await.unwrap().unwrap().username,
        "developer"
    );
    let requests = server.await.unwrap();
    assert!(
        requests
            .iter()
            .all(|request| request.starts_with("GET /api/v4/user ")
                && request.contains("Bearer test-pat"))
    );
}

#[tokio::test]
async fn expired_gitlab_credentials_rotate_before_loading_pipelines() {
    let (origin, server) = fixture(vec![
        ("200 OK", r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":7200,"token_type":"Bearer"}"#),
        ("200 OK", "[]"),
    ]).await;
    let service = Service {
        http: Ok(Http::fixture(&origin)),
        ..service()
    };
    let account = Account {
        provider: Provider::GitLab,
        host: "gitlab.com".to_owned(),
    };
    let mut old = credential();
    old.expires_at = Some(0);
    old.username = "developer".to_owned();
    service.write(account.clone(), old).await.unwrap();
    let loaded = service
        .load(Remote {
            host: account.host.clone(),
            provider: Some(account.provider),
            project: "group/sub/app".to_owned(),
        })
        .await;
    assert!(loaded.runs.unwrap().is_empty());
    assert_eq!(loaded.username.as_deref(), Some("developer"));
    let stored = service.read(account).await.unwrap().unwrap();
    assert_eq!(stored.access_token.expose(), "new-access");
    assert_eq!(stored.refresh_token.unwrap().expose(), "new-refresh");
    let requests = server.await.unwrap();
    assert!(requests[0].starts_with("POST /oauth/token "));
    assert!(requests[0].contains("refresh_token=test-refresh"));
    assert!(!requests[0].contains("client_secret"));
    assert!(
        requests[1].starts_with("GET /api/v4/projects/group%2Fsub%2Fapp/pipelines?per_page=30 ")
    );
    assert!(requests[1].contains("Bearer new-access"));
}

#[tokio::test]
async fn revoked_refresh_tokens_keep_the_account_visible_for_sign_out() {
    let (origin, server) = fixture(vec![("400 Bad Request", r#"{"error":"invalid_grant"}"#)]).await;
    let service = Service {
        http: Ok(Http::fixture(&origin)),
        ..service()
    };
    let account = account();
    let mut old = credential();
    old.expires_at = Some(0);
    old.username = "developer".to_owned();
    service.write(account.clone(), old).await.unwrap();
    let loaded = service
        .load(Remote {
            host: account.host,
            provider: Some(account.provider),
            project: "group/app".to_owned(),
        })
        .await;
    assert!(matches!(loaded.runs, Err(Failure::AuthExpired)));
    assert_eq!(loaded.username.as_deref(), Some("developer"));
    server.await.unwrap();
}

#[tokio::test]
async fn job_actions_require_credentials_and_refresh_expired_credentials_before_submission() {
    let remote = Remote {
        host: "gitlab.com".into(),
        provider: Some(Provider::GitLab),
        project: "group/app".into(),
    };
    let action = Action::Job(42, crate::domain::jobs::JobAction::Play);
    assert_eq!(
        service().control_job(remote.clone(), 500, action).await,
        Err(Failure::Access)
    );
    let (origin,server) = fixture(vec![
        ("200 OK", r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":7200,"token_type":"Bearer"}"#),
        ("200 OK", "{}"),
    ]).await;
    let service = Service {
        http: Ok(Http::fixture(&origin)),
        ..service()
    };
    let account = Account::from_remote(&remote).unwrap();
    let mut old = credential();
    old.expires_at = Some(0);
    service.write(account.clone(), old).await.unwrap();
    service.control_job(remote, 500, action).await.unwrap();
    let requests = server.await.unwrap();
    assert!(requests[0].starts_with("POST /oauth/token "));
    assert!(requests[1].starts_with("POST /api/v4/projects/group%2Fapp/jobs/42/play "));
    assert!(requests[1].contains("Bearer new-access"));
    assert_eq!(
        service
            .read(account)
            .await
            .unwrap()
            .unwrap()
            .access_token
            .expose(),
        "new-access"
    );
}

#[tokio::test]
async fn rejected_job_actions_do_not_refresh_and_resubmit_a_mutation() {
    let (origin, server) = fixture(vec![("401 Unauthorized", "{}")]).await;
    let service = Service {
        http: Ok(Http::fixture(&origin)),
        ..service()
    };
    let remote = Remote {
        host: "gitlab.com".into(),
        provider: Some(Provider::GitLab),
        project: "group/app".into(),
    };
    service
        .write(Account::from_remote(&remote).unwrap(), credential())
        .await
        .unwrap();
    assert_eq!(
        service
            .control_job(
                remote,
                500,
                Action::Job(42, crate::domain::jobs::JobAction::Play)
            )
            .await,
        Err(Failure::Access)
    );
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("POST /api/v4/projects/group%2Fapp/jobs/42/play "));
}

#[tokio::test]
async fn cancellation_during_vault_write_restores_the_previous_account() {
    struct CancelOnWrite {
        inner: MemoryStore,
        cancel: CancellationToken,
    }
    impl Store for CancelOnWrite {
        fn read(&self, account: &Account) -> Result<Option<Credential>, Failure> {
            self.inner.read(account)
        }
        fn write(&self, account: &Account, credential: &Credential) -> Result<(), Failure> {
            self.inner.write(account, credential)?;
            self.cancel.cancel();
            Ok(())
        }
        fn delete(&self, account: &Account) -> Result<(), Failure> {
            self.inner.delete(account)
        }
    }
    for previous in [None, Some(credential())] {
        let (origin, server) = fixture(vec![("200 OK", r#"{"username":"new-user"}"#)]).await;
        let cancel = CancellationToken::new();
        let store = Arc::new(CancelOnWrite {
            inner: MemoryStore::default(),
            cancel: cancel.clone(),
        });
        let account = account();
        if let Some(previous) = &previous {
            store.inner.write(&account, previous).unwrap();
        }
        let service = Service {
            http: Ok(Http::fixture(&origin)),
            store,
            gate: Arc::new(Mutex::new(())),
        };
        assert_eq!(
            service
                .connect_token(account.clone(), Secret::new("new-token".to_owned()), cancel)
                .await,
            Err(Failure::AuthCancelled)
        );
        let stored = service.read(account).await.unwrap();
        assert_eq!(
            stored.as_ref().map(|value| value.access_token.expose()),
            previous.as_ref().map(|value| value.access_token.expose())
        );
        server.await.unwrap();
    }
}
