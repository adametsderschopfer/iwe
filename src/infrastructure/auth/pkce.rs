use super::{AuthPrompt, Credential, Failure, Http, Secret, TokenResponse};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub(super) const REDIRECT_URI: &str = "http://127.0.0.1:46291/oauth/gitlab/callback";

#[derive(Clone)]
pub(crate) struct Session {
    base: String,
    client_id: String,
    state: Secret,
    verifier: Secret,
    authorization_url: String,
    listener: Arc<TcpListener>,
}

fn random_secret() -> Result<Secret, Failure> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Failure::Unavailable)?;
    Ok(Secret::new(URL_SAFE_NO_PAD.encode(bytes)))
}

fn challenge(verifier: &Secret) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose().as_bytes()))
}

pub(super) async fn start(base: &str, client_id: String) -> Result<Session, Failure> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 46291))
        .await
        .map_err(|_| Failure::AuthPortBusy)?;
    let state = random_secret()?;
    let verifier = random_secret()?;
    let mut url = url::Url::parse(&format!("{base}/oauth/authorize")).map_err(|_| Failure::Read)?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id.as_str()),
        ("redirect_uri", REDIRECT_URI),
        ("response_type", "code"),
        ("state", state.expose()),
        ("scope", "api"),
        ("code_challenge_method", "S256"),
        ("code_challenge", &challenge(&verifier)),
    ]);
    Ok(Session {
        base: base.to_owned(),
        client_id,
        state,
        verifier,
        authorization_url: url.to_string(),
        listener: Arc::new(listener),
    })
}

impl Session {
    pub(super) fn prompt(&self) -> AuthPrompt {
        AuthPrompt {
            url: self.authorization_url.clone(),
            code: None,
        }
    }

    pub(super) async fn complete(
        self,
        http: &Http,
        browser_message: &str,
    ) -> Result<Credential, Failure> {
        let code =
            tokio::time::timeout(Duration::from_secs(300), self.receive_code(browser_message))
                .await
                .map_err(|_| Failure::AuthExpired)??;
        let response = http
            .send(http.post(&format!("{}/oauth/token", self.base)).form(&[
                ("client_id", self.client_id.as_str()),
                ("code", code.expose()),
                ("grant_type", "authorization_code"),
                ("redirect_uri", REDIRECT_URI),
                ("code_verifier", self.verifier.expose()),
            ]))
            .await?;
        let tokens: TokenResponse = response.json()?;
        if tokens.error.is_none() {
            response.check(false)?;
        }
        tokens.credential(&self.client_id)
    }

    async fn receive_code(&self, browser_message: &str) -> Result<Secret, Failure> {
        loop {
            let (mut socket, peer) = self
                .listener
                .accept()
                .await
                .map_err(|_| Failure::Unavailable)?;
            if !peer.ip().is_loopback() {
                continue;
            }
            let mut bytes = zeroize::Zeroizing::new(Vec::new());
            let read = async {
                loop {
                    let mut buffer = [0_u8; 1024];
                    let count = socket.read(&mut buffer).await.map_err(|_| Failure::Read)?;
                    if count == 0 {
                        return Err(Failure::Read);
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > 8192 {
                        return Err(Failure::Read);
                    }
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        return Ok(());
                    }
                }
            };
            if !matches!(
                tokio::time::timeout(Duration::from_secs(2), read).await,
                Ok(Ok(()))
            ) {
                continue;
            }
            let result = parse_callback(&bytes, &self.state);
            let valid = matches!(result, Ok(Some(_)) | Err(Failure::AuthDenied));
            let body = if valid { browser_message } else { "" };
            let status = if valid { "200 OK" } else { "400 Bad Request" };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = tokio::time::timeout(
                Duration::from_secs(2),
                socket.write_all(response.as_bytes()),
            )
            .await;
            match result {
                Ok(Some(code)) => return Ok(code),
                Err(Failure::AuthDenied) => return Err(Failure::AuthDenied),
                _ => {}
            }
        }
    }
}

fn parse_callback(bytes: &[u8], expected_state: &Secret) -> Result<Option<Secret>, Failure> {
    let request = std::str::from_utf8(bytes).map_err(|_| Failure::Read)?;
    let mut lines = request.split("\r\n");
    let parts: Vec<_> = lines
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    if parts.len() != 3
        || parts[0] != "GET"
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].starts_with("//")
    {
        return Ok(None);
    }
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
        .collect();
    if hosts.len() != 1 || hosts[0].1.trim() != "127.0.0.1:46291" {
        return Ok(None);
    }
    let url = url::Url::parse(&format!("http://127.0.0.1:46291{}", parts[1]))
        .map_err(|_| Failure::Read)?;
    if url.path() != "/oauth/gitlab/callback" || url.fragment().is_some() {
        return Ok(None);
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    for name in ["state", "code", "error"] {
        if pairs.iter().filter(|(key, _)| key == name).count() > 1 {
            return Ok(None);
        }
    }
    let unique = |name: &str| {
        let values: Vec<_> = pairs
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.as_ref())
            .collect();
        if values.len() == 1 {
            Some(values[0])
        } else {
            None
        }
    };
    if unique("state") != Some(expected_state.expose()) {
        return Ok(None);
    }
    if unique("error").is_some() && unique("code").is_none() {
        return Err(Failure::AuthDenied);
    }
    if unique("error").is_some() {
        return Ok(None);
    }
    Ok(unique("code")
        .filter(|code| !code.is_empty() && code.len() <= 4096)
        .map(|code| Secret::new(code.to_owned())))
}

#[cfg(test)]
mod tests;
