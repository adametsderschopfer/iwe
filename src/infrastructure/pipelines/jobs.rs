//! Load complete, bounded job lists and send explicit provider actions.

use super::super::http::Http;
use crate::domain::{
    Failure,
    auth::{Account, Credential},
    jobs::{Action, Details, Job, JobAction, Stage},
    repository::{Pipeline, Provider, Remote},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Duration};

const PAGE_SIZE: usize = 100;
const MAX_JOBS: usize = 1_000;
const GITLAB_JOBS: &str = r#"
query PipelineJobs($project: ID!, $iid: ID!, $after: String) {
  project(fullPath: $project) {
    pipeline(iid: $iid) {
      id cancelable
      stages(first: 100) { nodes { name } pageInfo { hasNextPage } }
      jobs(first: 100, after: $after, retried: false) {
        nodes { id name status webPath playable retryable cancelable stage { name } }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"#;

pub(crate) async fn load(
    http: &Http,
    remote: &Remote,
    run: &Pipeline,
    credential: Option<&Credential>,
) -> Result<Details, Failure> {
    let account = Account::from_remote(remote)?;
    let operation = async {
        match account.provider {
            Provider::GitLab => gitlab(http, remote, &account, run, credential).await,
            Provider::GitHub => github(http, remote, &account, run, credential).await,
        }
    };
    tokio::time::timeout(Duration::from_secs(45), operation)
        .await
        .map_err(|_| Failure::Unavailable)?
}

async fn github(
    http: &Http,
    remote: &Remote,
    account: &Account,
    run: &Pipeline,
    credential: Option<&Credential>,
) -> Result<Details, Failure> {
    let mut jobs = Vec::new();
    let mut ids = BTreeSet::new();
    for page in 1..=MAX_JOBS / PAGE_SIZE {
        let mut url = api_url(
            account,
            &remote.project,
            &["runs", &run.id.to_string(), "jobs"],
        )?;
        url.query_pairs_mut().extend_pairs([
            ("filter", "latest"),
            ("per_page", "100"),
            ("page", &page.to_string()),
        ]);
        let request = http
            .get(url.as_str(), credential.map(|value| &value.access_token))?
            .header("X-GitHub-Api-Version", "2022-11-28");
        let response = http.send(request).await?;
        response.check(credential.is_some())?;
        let value: Value = response.json()?;
        let total = value
            .get("total_count")
            .and_then(Value::as_u64)
            .ok_or(Failure::Read)?;
        if total > MAX_JOBS as u64 {
            return Err(Failure::TooLarge);
        }
        let values = array(&value, "jobs")?;
        if values.len() > PAGE_SIZE {
            return Err(Failure::TooLarge);
        }
        for value in values {
            let id = value
                .get("id")
                .and_then(Value::as_u64)
                .ok_or(Failure::Read)?;
            if !ids.insert(id) {
                return Err(Failure::Read);
            }
            let status = field(value, "status")?;
            let conclusion = value.get("conclusion").and_then(Value::as_str);
            jobs.push(Job {
                id,
                name: field(value, "name")?.to_owned(),
                status: conclusion
                    .filter(|value| !value.is_empty())
                    .unwrap_or(status)
                    .to_owned(),
                url: field(value, "html_url")?.to_owned(),
                actions: (status == "completed"
                    && conclusion.is_some_and(|value| !matches!(value, "skipped" | "stale")))
                .then_some(JobAction::Retry)
                .into_iter()
                .collect(),
            });
        }
        if jobs.len() as u64 >= total {
            return Ok(Details {
                stages: vec![Stage { name: None, jobs }],
                cancelable: run.is_active(),
            });
        }
        if values.is_empty() {
            return Err(Failure::Read);
        }
    }
    Err(Failure::TooLarge)
}

async fn gitlab(
    http: &Http,
    remote: &Remote,
    account: &Account,
    run: &Pipeline,
    credential: Option<&Credential>,
) -> Result<Details, Failure> {
    let token = credential.map(|value| &value.access_token);
    let iid = if let Some(number) = run.number {
        number
    } else {
        let url = api_url(
            account,
            &remote.project,
            &["pipelines", &run.id.to_string()],
        )?;
        let response = http.send(http.get(url.as_str(), token)?).await?;
        response.check(credential.is_some())?;
        response
            .json::<Value>()?
            .get("iid")
            .and_then(Value::as_u64)
            .ok_or(Failure::Read)?
    };
    let url = format!("{}/api/graphql", account.base_url());
    let mut details = Details::default();
    let mut cursor: Option<String> = None;
    let mut cursors = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for page in 0..MAX_JOBS / PAGE_SIZE {
        let request = http.post_authenticated(&url, token)?.json(&json!({
            "query": GITLAB_JOBS,
            "variables": { "project": remote.project, "iid": iid.to_string(), "after": cursor },
        }));
        let response = http.send(request).await?;
        response.check(credential.is_some())?;
        let value: Value = response.json()?;
        if value
            .get("errors")
            .is_some_and(|errors| errors.as_array().is_none_or(|errors| !errors.is_empty()))
        {
            return Err(Failure::Unavailable);
        }
        let pipeline = value
            .pointer("/data/project/pipeline")
            .filter(|value| value.is_object())
            .ok_or(if credential.is_some() {
                Failure::PermissionDenied
            } else {
                Failure::Access
            })?;
        if field(pipeline, "id")? != format!("gid://gitlab/Ci::Pipeline/{}", run.id) {
            return Err(Failure::Read);
        }
        if page == 0 {
            if pipeline
                .pointer("/stages/pageInfo/hasNextPage")
                .and_then(Value::as_bool)
                != Some(false)
            {
                return Err(Failure::TooLarge);
            }
            details.cancelable = pipeline
                .get("cancelable")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let stages = array(pipeline.get("stages").ok_or(Failure::Read)?, "nodes")?;
            if stages.len() > PAGE_SIZE {
                return Err(Failure::TooLarge);
            }
            let mut names = BTreeSet::new();
            for stage in stages {
                let name = field(stage, "name")?.to_owned();
                if !names.insert(name.clone()) {
                    return Err(Failure::Read);
                }
                details.stages.push(Stage {
                    name: Some(name),
                    jobs: Vec::new(),
                });
            }
        }
        let connection = pipeline.get("jobs").ok_or(Failure::Read)?;
        let values = array(connection, "nodes")?;
        if values.len() > PAGE_SIZE {
            return Err(Failure::TooLarge);
        }
        for value in values {
            let id = field(value, "id")?
                .rsplit('/')
                .next()
                .and_then(|id| id.parse::<u64>().ok())
                .ok_or(Failure::Read)?;
            if !ids.insert(id) {
                return Err(Failure::Read);
            }
            let name = field(value.get("stage").ok_or(Failure::Read)?, "name")?;
            let stage = details
                .stages
                .iter_mut()
                .find(|stage| stage.name.as_deref() == Some(name))
                .ok_or(Failure::Read)?;
            let path = field(value, "webPath")?;
            let url = url::Url::parse(&account.base_url())
                .map_err(|_| Failure::Read)?
                .join(path)
                .map_err(|_| Failure::Read)?;
            stage.jobs.push(Job {
                id,
                name: field(value, "name")?.to_owned(),
                status: field(value, "status")?.to_ascii_lowercase(),
                url: url.to_string(),
                actions: [
                    ("playable", JobAction::Play),
                    ("retryable", JobAction::Retry),
                    ("cancelable", JobAction::Cancel),
                ]
                .into_iter()
                .filter_map(|(field, action)| {
                    (value.get(field).and_then(Value::as_bool) == Some(true)).then_some(action)
                })
                .collect(),
            });
        }
        let page_info = connection.get("pageInfo").ok_or(Failure::Read)?;
        match page_info.get("hasNextPage").and_then(Value::as_bool) {
            Some(false) => {
                for stage in &mut details.stages {
                    stage.jobs.sort_by(|left, right| {
                        left.name.cmp(&right.name).then(left.id.cmp(&right.id))
                    });
                }
                return Ok(details);
            }
            Some(true) if !values.is_empty() => {
                let next = field(page_info, "endCursor")?.to_owned();
                if next.is_empty() || !cursors.insert(next.clone()) {
                    return Err(Failure::Read);
                }
                cursor = Some(next);
            }
            _ => return Err(Failure::Read),
        }
    }
    Err(Failure::TooLarge)
}

pub(crate) async fn control(
    http: &Http,
    remote: &Remote,
    pipeline: u64,
    action: Action,
    credential: &Credential,
) -> Result<(), Failure> {
    let account = Account::from_remote(remote)?;
    let (id, resource, operation) = match action {
        Action::Job(id, action) if action.supported(account.provider) => (
            id,
            "jobs",
            match (account.provider, action) {
                (_, JobAction::Play) => "play",
                (_, JobAction::Cancel) => "cancel",
                (Provider::GitLab, JobAction::Retry) => "retry",
                (Provider::GitHub, JobAction::Retry) => "rerun",
            },
        ),
        Action::CancelPipeline => (
            pipeline,
            if account.provider == Provider::GitHub {
                "runs"
            } else {
                "pipelines"
            },
            "cancel",
        ),
        _ => return Err(Failure::Unavailable),
    };
    let url = api_url(
        &account,
        &remote.project,
        &[resource, &id.to_string(), operation],
    )?;
    let mut request = http.post_authenticated(url.as_str(), Some(&credential.access_token))?;
    if account.provider == Provider::GitHub {
        request = request.header("X-GitHub-Api-Version", "2022-11-28");
    }
    http.send(request).await?.check(true)
}

fn api_url(account: &Account, project: &str, suffix: &[&str]) -> Result<url::Url, Failure> {
    let mut url =
        url::Url::parse(&super::endpoint(account, project)?).map_err(|_| Failure::Read)?;
    url.set_query(None);
    url.path_segments_mut()
        .map_err(|_| Failure::Read)?
        .pop()
        .extend(suffix);
    Ok(url)
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, Failure> {
    value.get(name).and_then(Value::as_str).ok_or(Failure::Read)
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, Failure> {
    value
        .get(name)
        .and_then(Value::as_array)
        .ok_or(Failure::Read)
}

#[cfg(test)]
mod tests;
