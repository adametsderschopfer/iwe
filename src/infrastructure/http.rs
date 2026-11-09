//! Bound provider responses and forbid redirects of authenticated requests.

use crate::domain::{Failure, auth::Secret};
use reqwest::{
    Client, RequestBuilder, StatusCode,
    header::{AUTHORIZATION, HeaderValue},
};
use serde::de::DeserializeOwned;
use std::time::Duration;
use zeroize::Zeroizing;

#[derive(Clone)]
pub(super) struct Http {
    client: Client,
    #[cfg(test)]
    origin: Option<url::Url>,
}

pub(super) struct Response {
    pub status: StatusCode,
    pub limited: bool,
    pub bytes: Zeroizing<Vec<u8>>,
}

impl Response {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Failure> {
        serde_json::from_slice(&self.bytes).map_err(|_| Failure::Read)
    }
    pub fn check(&self, authenticated: bool) -> Result<(), Failure> {
        if self.status.is_success() {
            return Ok(());
        }
        if self.limited {
            return Err(Failure::RateLimited);
        }
        match self.status.as_u16() {
            401 => Err(Failure::Access),
            403 | 404 if authenticated => Err(Failure::PermissionDenied),
            403 | 404 => Err(Failure::Access),
            _ => Err(Failure::Unavailable),
        }
    }
}

impl Http {
    pub fn new() -> Result<Self, Failure> {
        Self::build(true)
    }

    fn build(https_only: bool) -> Result<Self, Failure> {
        let client = Client::builder()
            .https_only(https_only)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(25))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(concat!("IWE/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Failure::Unavailable)?;
        Ok(Self {
            client,
            #[cfg(test)]
            origin: None,
        })
    }

    pub fn get(&self, url: &str, token: Option<&Secret>) -> Result<RequestBuilder, Failure> {
        Self::authorize(
            self.client.get(url).header("Accept", "application/json"),
            token,
        )
    }

    pub fn post_authenticated(
        &self,
        url: &str,
        token: Option<&Secret>,
    ) -> Result<RequestBuilder, Failure> {
        Self::authorize(self.post(url), token)
    }

    fn authorize(
        mut request: RequestBuilder,
        token: Option<&Secret>,
    ) -> Result<RequestBuilder, Failure> {
        if let Some(token) = token {
            let mut value = HeaderValue::from_str(&format!("Bearer {}", token.expose()))
                .map_err(|_| Failure::InvalidToken)?;
            value.set_sensitive(true);
            request = request.header(AUTHORIZATION, value);
        }
        Ok(request)
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(url).header("Accept", "application/json")
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response, Failure> {
        let request = request.build().map_err(|_| Failure::Unavailable)?;
        #[cfg(test)]
        let request = {
            let mut request = request;
            if let Some(origin) = &self.origin {
                let mut target = origin.clone();
                target.set_path(request.url().path());
                target.set_query(request.url().query());
                *request.url_mut() = target;
            }
            request
        };
        let mut response = self
            .client
            .execute(request)
            .await
            .map_err(|_| Failure::Unavailable)?;
        let status = response.status();
        let limited = status == StatusCode::TOO_MANY_REQUESTS
            || (status == StatusCode::FORBIDDEN
                && response
                    .headers()
                    .get("x-ratelimit-remaining")
                    .is_some_and(|value| value == "0"));
        if response
            .content_length()
            .is_some_and(|length| length > 4 * 1024 * 1024)
        {
            return Err(Failure::TooLarge);
        }
        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Unavailable)? {
            if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
                return Err(Failure::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Response {
            status,
            limited,
            bytes,
        })
    }

    #[cfg(test)]
    pub(super) fn local() -> Self {
        Self::build(false).unwrap()
    }

    #[cfg(test)]
    pub(super) fn fixture(origin: &str) -> Self {
        Self {
            origin: Some(url::Url::parse(origin).unwrap()),
            ..Self::local()
        }
    }
}

#[cfg(test)]
mod tests;
