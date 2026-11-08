//! Keep account identity and redacted credentials separate from workspace metadata.

use super::{
    Failure,
    repository::{Provider, Remote},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use zeroize::Zeroizing;

#[derive(Clone, Default)]
pub(crate) struct Secret(Zeroizing<String>);

impl Secret {
    pub(crate) fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub(crate) fn expose(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.expose())
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Account {
    pub provider: Provider,
    pub host: String,
}

impl Account {
    pub(crate) fn from_remote(remote: &Remote) -> Result<Self, Failure> {
        let provider = remote.provider.ok_or(Failure::Unavailable)?;
        let url =
            url::Url::parse(&format!("https://{}", remote.host)).map_err(|_| Failure::Read)?;
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !remote
                .host
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".-:".contains(&byte))
        {
            return Err(Failure::Read);
        }
        let host = url.host_str().ok_or(Failure::Read)?.to_ascii_lowercase();
        let host = url
            .port()
            .map(|port| format!("{host}:{port}"))
            .unwrap_or(host);
        Ok(Self { provider, host })
    }

    pub(crate) fn browser_login(&self) -> bool {
        matches!(
            (self.provider, self.host.as_str()),
            (Provider::GitHub, "github.com") | (Provider::GitLab, "gitlab.com")
        )
    }

    pub(crate) fn storage_key(&self) -> String {
        format!("{}@{}", self.provider.name(), self.host)
    }

    pub(crate) fn base_url(&self) -> String {
        format!("https://{}", self.host)
    }

    pub(crate) fn api_base(&self) -> String {
        match self.provider {
            Provider::GitHub if self.host == "github.com" => "https://api.github.com".to_owned(),
            Provider::GitHub => format!("{}/api/v3", self.base_url()),
            Provider::GitLab => format!("{}/api/v4", self.base_url()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Credential {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_at: Option<u64>,
    pub client_id: Option<String>,
    pub username: String,
}

#[derive(Clone)]
pub(crate) struct AuthPrompt {
    pub url: String,
    pub code: Option<String>,
}

impl fmt::Debug for AuthPrompt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthPrompt")
    }
}

#[cfg(test)]
mod tests;
