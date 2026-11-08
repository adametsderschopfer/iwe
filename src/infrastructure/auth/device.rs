use super::{AuthPrompt, Credential, Failure, Http, Secret, TokenResponse, oauth_error};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Clone)]
pub(crate) struct Session {
    base: String,
    client_id: String,
    device_code: Secret,
    user_code: String,
    verification_uri: String,
    deadline: Instant,
    interval: u64,
}

pub(super) async fn start(http: &Http, base: &str, client_id: String) -> Result<Session, Failure> {
    let response = http
        .send(
            http.post(&format!("{base}/login/device/code"))
                .form(&[("client_id", client_id.as_str()), ("scope", "repo")]),
        )
        .await?;
    #[derive(Deserialize)]
    struct Device {
        device_code: Secret,
        user_code: String,
        verification_uri: String,
        expires_in: u64,
        interval: Option<u64>,
    }
    if !response.status.is_success() {
        if let Ok(error) = response.json::<TokenResponse>()
            && let Some(error) = error.error
        {
            return Err(oauth_error(&error));
        }
        response.check(false)?;
    }
    let payload: Device = response.json()?;
    // Never follow a verification URL supplied by a different origin.
    if payload.verification_uri != format!("{base}/login/device")
        || payload.user_code.is_empty()
        || payload.user_code.len() > 128
        || payload.device_code.is_empty()
        || payload.device_code.expose().len() > 8192
        || payload.expires_in == 0
    {
        return Err(Failure::Read);
    }
    Ok(Session {
        base: base.to_owned(),
        client_id,
        device_code: payload.device_code,
        user_code: payload.user_code,
        verification_uri: payload.verification_uri,
        deadline: Instant::now() + Duration::from_secs(payload.expires_in.min(900)),
        interval: payload.interval.unwrap_or(5).clamp(1, 900),
    })
}

impl Session {
    pub(super) fn prompt(&self) -> AuthPrompt {
        AuthPrompt {
            url: self.verification_uri.clone(),
            code: Some(self.user_code.clone()),
        }
    }

    pub(super) async fn complete(mut self, http: &Http) -> Result<Credential, Failure> {
        loop {
            let next = Instant::now() + Duration::from_secs(self.interval);
            if next >= self.deadline {
                return Err(Failure::AuthExpired);
            }
            tokio::time::sleep_until(next).await;
            let request = http
                .post(&format!("{}/login/oauth/access_token", self.base))
                .form(&[
                    ("client_id", self.client_id.as_str()),
                    ("device_code", self.device_code.expose()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ]);
            let response = tokio::time::timeout_at(self.deadline, http.send(request))
                .await
                .map_err(|_| Failure::AuthExpired)??;
            let tokens: TokenResponse = response.json()?;
            match tokens.error.as_deref() {
                Some("authorization_pending") => continue,
                Some("slow_down") => {
                    self.interval = self.interval.saturating_add(5);
                    continue;
                }
                Some(error) => return Err(oauth_error(error)),
                None => {
                    response.check(false)?;
                    return tokens.credential(&self.client_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
