//! Access pipeline APIs directly without installing provider command-line tools.

pub(crate) mod jobs;

use crate::domain::{
    Failure,
    auth::{Account, Credential},
    repository::{Pipeline, Provider, Remote},
};
use crate::infrastructure::http::Http;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct Loaded {
    pub runs: Result<Vec<Pipeline>, Failure>,
    pub username: Option<String>,
}

pub(crate) async fn load(
    http: &Http,
    remote: &Remote,
    credential: Option<&Credential>,
) -> Result<Vec<Pipeline>, Failure> {
    let account = Account::from_remote(remote)?;
    let endpoint = endpoint(&account, &remote.project)?;
    let request = http.get(&endpoint, credential.map(|value| &value.access_token))?;
    let request = if account.provider == Provider::GitHub {
        request.header("X-GitHub-Api-Version", "2022-11-28")
    } else {
        request
    };
    let response = http.send(request).await?;
    response.check(credential.is_some())?;
    parse(&response.bytes, account.provider)
}

fn endpoint(account: &Account, project: &str) -> Result<String, Failure> {
    let mut url = url::Url::parse(&account.api_base()).map_err(|_| Failure::Read)?;
    {
        let mut path = url.path_segments_mut().map_err(|_| Failure::Read)?;
        path.pop_if_empty();
        match account.provider {
            Provider::GitHub => {
                let parts: Vec<_> = project.split('/').collect();
                if parts.len() != 2
                    || parts
                        .iter()
                        .any(|part| part.is_empty() || *part == "." || *part == "..")
                {
                    return Err(Failure::Read);
                }
                path.extend(["repos", parts[0], parts[1], "actions", "runs"]);
            }
            Provider::GitLab => {
                path.extend(["projects", project, "pipelines"]);
            }
        }
    }
    url.query_pairs_mut().append_pair("per_page", "30");
    Ok(url.to_string())
}

fn parse(bytes: &[u8], provider: Provider) -> Result<Vec<Pipeline>, Failure> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Failure::Read)?;
    let values = if provider == Provider::GitHub {
        value.get("workflow_runs").and_then(Value::as_array)
    } else {
        value.as_array()
    }
    .ok_or(Failure::Read)?;
    values
        .iter()
        .take(30)
        .map(|value| {
            let field = |key| {
                value
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let id = value
                .get("id")
                .and_then(Value::as_u64)
                .ok_or(Failure::Read)?;
            let github = provider == Provider::GitHub;
            let conclusion = field("conclusion");
            let sha = field(if github { "head_sha" } else { "sha" });
            Ok(Pipeline {
                id,
                number: value
                    .get(if github { "run_number" } else { "iid" })
                    .and_then(Value::as_u64),
                title: if github {
                    field("display_title")
                } else {
                    field("name")
                },
                branch: field(if github { "head_branch" } else { "ref" }),
                commit: (matches!(sha.len(), 40 | 64)
                    && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
                .then_some(sha),
                status: if conclusion.is_empty() {
                    field("status")
                } else {
                    conclusion
                },
                date: field("created_at"),
                url: field(if github { "html_url" } else { "web_url" }),
            })
        })
        .collect()
}

pub(crate) async fn open_url(url: String) -> Result<(), Failure> {
    let parsed = url::Url::parse(&url).map_err(|_| Failure::Read)?;
    if !matches!(parsed.scheme(), "https" | "http")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(Failure::Read);
    }
    tokio::task::spawn_blocking(move || open::that(url).map_err(|_| Failure::Unavailable))
        .await
        .map_err(|_| Failure::Unavailable)?
}

#[cfg(test)]
mod tests;
