//! Coordinate browser authorization, token refresh, and serialized credential operations.

mod device;
mod pkce;

use super::{
    credentials::{FileStore, SessionStore, Store},
    http::Http,
    pipelines,
};
use crate::domain::{
    Failure,
    auth::{Account, AuthPrompt, Credential, Secret},
    jobs::{Action, Details},
    repository::{Pipeline, Provider, Remote},
};
use serde::Deserialize;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct Service {
    http: Result<Http, Failure>,
    store: Arc<SessionStore>,
    gate: Arc<Mutex<()>>,
}

#[derive(Clone)]
pub(crate) enum Pending {
    GitHub(device::Session),
    GitLab(pkce::Session),
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PendingAuthorization")
    }
}

impl Pending {
    pub(crate) fn prompt(&self) -> AuthPrompt {
        match self {
            Self::GitHub(session) => session.prompt(),
            Self::GitLab(session) => session.prompt(),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<Secret>,
    refresh_token: Option<Secret>,
    expires_in: Option<u64>,
    token_type: Option<String>,
    error: Option<String>,
}

impl TokenResponse {
    fn credential(self, client_id: &str) -> Result<Credential, Failure> {
        if let Some(error) = self.error {
            return Err(oauth_error(&error));
        }
        if self
            .token_type
            .as_ref()
            .is_some_and(|kind| !kind.eq_ignore_ascii_case("bearer"))
        {
            return Err(Failure::Read);
        }
        let access_token = self
            .access_token
            .filter(|token| !token.is_empty())
            .ok_or(Failure::Read)?;
        Ok(Credential {
            access_token,
            refresh_token: self.refresh_token,
            expires_at: self.expires_in.map(|seconds| now().saturating_add(seconds)),
            client_id: Some(client_id.to_owned()),
            username: String::new(),
        })
    }
}

fn oauth_error(error: &str) -> Failure {
    match error {
        "access_denied" => Failure::AuthDenied,
        "expired_token" | "invalid_grant" | "bad_refresh_token" => Failure::AuthExpired,
        "incorrect_client_credentials"
        | "invalid_client"
        | "device_flow_disabled"
        | "unauthorized_client" => Failure::AuthNotConfigured,
        _ => Failure::Unavailable,
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn client_id(provider: Provider) -> Result<String, Failure> {
    let (name, bundled) = match provider {
        Provider::GitHub => ("IWE_GITHUB_CLIENT_ID", option_env!("IWE_GITHUB_CLIENT_ID")),
        Provider::GitLab => ("IWE_GITLAB_CLIENT_ID", option_env!("IWE_GITLAB_CLIENT_ID")),
    };
    std::env::var(name)
        .ok()
        .or_else(|| bundled.map(str::to_owned))
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
        .ok_or(Failure::AuthNotConfigured)
}

impl Service {
    pub(crate) fn new() -> Self {
        Self {
            http: Http::new(),
            store: Arc::new(SessionStore::new(Arc::new(FileStore))),
            gate: Arc::new(Mutex::new(())),
        }
    }

    fn http(&self) -> Result<&Http, Failure> {
        self.http.as_ref().map_err(|error| *error)
    }

    pub(crate) async fn start(&self, account: Account) -> Result<Pending, Failure> {
        if !account.browser_login() {
            return Err(Failure::AuthNotConfigured);
        }
        let client_id = client_id(account.provider)?;
        match account.provider {
            Provider::GitHub => device::start(self.http()?, &account.base_url(), client_id)
                .await
                .map(Pending::GitHub),
            Provider::GitLab => pkce::start(&account.base_url(), client_id)
                .await
                .map(Pending::GitLab),
        }
    }

    pub(crate) async fn finish(
        &self,
        account: Account,
        pending: Pending,
        cancel: CancellationToken,
        browser_message: String,
    ) -> Result<String, Failure> {
        let operation = async {
            Ok(match pending {
                Pending::GitHub(session) => session.complete(self.http()?).await?,
                Pending::GitLab(session) => {
                    session.complete(self.http()?, &browser_message).await?
                }
            })
        };
        let credential = tokio::select! { biased; _ = cancel.cancelled() => return Err(Failure::AuthCancelled), result = operation => result? };
        self.validate_and_save(account, credential, &cancel).await
    }

    pub(crate) async fn connect_token(
        &self,
        account: Account,
        token: Secret,
        cancel: CancellationToken,
    ) -> Result<String, Failure> {
        if account.browser_login() {
            return Err(Failure::AuthNotConfigured);
        }
        if token.is_empty()
            || token.expose().len() > 8192
            || token
                .expose()
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(Failure::InvalidToken);
        }
        let credential = Credential {
            access_token: token,
            refresh_token: None,
            expires_at: None,
            client_id: None,
            username: String::new(),
        };
        self.validate_and_save(account, credential, &cancel).await
    }

    async fn validate_and_save(
        &self,
        account: Account,
        mut credential: Credential,
        cancel: &CancellationToken,
    ) -> Result<String, Failure> {
        let _guard = tokio::select! { biased; _ = cancel.cancelled() => return Err(Failure::AuthCancelled), guard = self.gate.lock() => guard };
        if cancel.is_cancelled() {
            return Err(Failure::AuthCancelled);
        }
        credential.username = tokio::select! { biased; _ = cancel.cancelled() => return Err(Failure::AuthCancelled), result = self.validate(&account, &credential.access_token) => result? };
        if cancel.is_cancelled() {
            return Err(Failure::AuthCancelled);
        }
        let username = credential.username.clone();
        self.allow_credential_retry(account.clone()).await?;
        if cancel.is_cancelled() {
            return Err(Failure::AuthCancelled);
        }
        let previous = self.read(account.clone()).await?;
        if cancel.is_cancelled() {
            return Err(Failure::AuthCancelled);
        }
        // Keep the gate until blocking storage finishes, even when the dialog closes.
        self.write(account.clone(), credential).await?;
        if cancel.is_cancelled() {
            if let Some(previous) = previous {
                self.write(account, previous).await?;
            } else {
                self.delete(account).await?;
            }
            return Err(Failure::AuthCancelled);
        }
        Ok(username)
    }

    async fn validate(&self, account: &Account, token: &Secret) -> Result<String, Failure> {
        let response = self
            .http()?
            .send(
                self.http()?
                    .get(&format!("{}/user", account.api_base()), Some(token))?,
            )
            .await?;
        response.check(true).map_err(|error| {
            if error == Failure::Access {
                Failure::InvalidToken
            } else {
                error
            }
        })?;
        #[derive(Deserialize)]
        struct User {
            login: Option<String>,
            username: Option<String>,
        }
        let user: User = response.json()?;
        match account.provider {
            Provider::GitHub => user.login,
            Provider::GitLab => user.username,
        }
        .filter(|value| !value.is_empty())
        .ok_or(Failure::Read)
    }

    async fn read(&self, account: Account) -> Result<Option<Credential>, Failure> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.read(&account))
            .await
            .map_err(|_| Failure::Credentials)?
    }

    pub(crate) async fn retry_credentials(&self, remote: &Remote) -> Result<(), Failure> {
        let account = Account::from_remote(remote)?;
        let _guard = self.gate.lock().await;
        self.allow_credential_retry(account).await
    }

    async fn allow_credential_retry(&self, account: Account) -> Result<(), Failure> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.allow_retry(&account))
            .await
            .map_err(|_| Failure::Credentials)?
    }

    async fn write(&self, account: Account, credential: Credential) -> Result<(), Failure> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.write(&account, &credential))
            .await
            .map_err(|_| Failure::Credentials)?
    }

    pub(crate) async fn disconnect(&self, account: Account) -> Result<(), Failure> {
        let _guard = self.gate.lock().await;
        self.delete(account).await
    }

    async fn delete(&self, account: Account) -> Result<(), Failure> {
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.delete(&account))
            .await
            .map_err(|_| Failure::Credentials)?
    }

    pub(crate) async fn load(&self, remote: Remote) -> pipelines::Loaded {
        let mut username = None;
        let runs = self.load_runs(remote, &mut username).await;
        pipelines::Loaded { runs, username }
    }

    async fn load_runs(
        &self,
        remote: Remote,
        username: &mut Option<String>,
    ) -> Result<Vec<crate::domain::repository::Pipeline>, Failure> {
        let account = Account::from_remote(&remote)?;
        // Serialize refresh rotation and disconnect to prevent restored credentials after logout.
        let _guard = self.gate.lock().await;
        let credential = self.read(account.clone()).await?;
        *username = credential.as_ref().map(|value| value.username.clone());
        let credential = self.refresh_if_expired(&account, credential).await?;
        let result = pipelines::load(self.http()?, &remote, credential.as_ref()).await;
        if matches!(result, Err(Failure::Access))
            && let Some(current) = &credential
            && current.refresh_token.is_some()
        {
            let refreshed = self.refresh(&account, current).await?;
            return pipelines::load(self.http()?, &remote, Some(&refreshed)).await;
        }
        result
    }

    pub(crate) async fn load_jobs(
        &self,
        remote: Remote,
        run: Pipeline,
    ) -> Result<Details, Failure> {
        let account = Account::from_remote(&remote)?;
        let _guard = self.gate.lock().await;
        let credential = self.read(account.clone()).await?;
        let credential = self.refresh_if_expired(&account, credential).await?;
        let result = pipelines::jobs::load(self.http()?, &remote, &run, credential.as_ref()).await;
        if matches!(result, Err(Failure::Access))
            && let Some(current) = &credential
            && current.refresh_token.is_some()
        {
            let refreshed = self.refresh(&account, current).await?;
            return pipelines::jobs::load(self.http()?, &remote, &run, Some(&refreshed)).await;
        }
        result
    }

    pub(crate) async fn control_job(
        &self,
        remote: Remote,
        run: u64,
        action: Action,
    ) -> Result<(), Failure> {
        let account = Account::from_remote(&remote)?;
        let _guard = self.gate.lock().await;
        let credential = self.read(account.clone()).await?;
        let credential = self
            .refresh_if_expired(&account, credential)
            .await?
            .ok_or(Failure::Access)?;
        // Never retry a mutation: a lost response does not mean the provider rejected the action.
        pipelines::jobs::control(self.http()?, &remote, run, action, &credential).await
    }

    async fn refresh_if_expired(
        &self,
        account: &Account,
        mut credential: Option<Credential>,
    ) -> Result<Option<Credential>, Failure> {
        if let Some(current) = &credential
            && current
                .expires_at
                .is_some_and(|expires| expires <= now().saturating_add(60))
        {
            credential = Some(self.refresh(account, current).await?);
        }
        Ok(credential)
    }

    async fn refresh(
        &self,
        account: &Account,
        credential: &Credential,
    ) -> Result<Credential, Failure> {
        let refresh = credential
            .refresh_token
            .as_ref()
            .ok_or(Failure::AuthExpired)?;
        let client_id = credential
            .client_id
            .as_deref()
            .ok_or(Failure::AuthExpired)?;
        let endpoint = match account.provider {
            Provider::GitHub => "/login/oauth/access_token",
            Provider::GitLab => "/oauth/token",
        };
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh.expose()),
        ];
        if account.provider == Provider::GitLab {
            form.push(("redirect_uri", pkce::REDIRECT_URI));
        }
        let response = self
            .http()?
            .send(
                self.http()?
                    .post(&format!("{}{endpoint}", account.base_url()))
                    .form(&form),
            )
            .await?;
        let tokens: TokenResponse = response.json()?;
        if tokens.error.is_none() {
            response.check(true)?;
        }
        let mut refreshed = tokens.credential(client_id)?;
        if refreshed.refresh_token.is_none() {
            refreshed.refresh_token = credential.refresh_token.clone();
        }
        refreshed.username = credential.username.clone();
        self.write(account.clone(), refreshed.clone()).await?;
        Ok(refreshed)
    }
}

#[cfg(test)]
mod tests;
