use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Provider {
    GitHub,
    GitLab,
}

impl Provider {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Remote {
    pub host: String,
    pub project: String,
    pub provider: Option<Provider>,
}

impl Remote {
    pub(crate) fn parse(input: &str) -> Option<Self> {
        let normalized = if input.contains("://") {
            input.to_owned()
        } else {
            let (host, path) = input.split_once(':')?;
            if host.contains('/') || host.contains('\\') {
                return None;
            }
            format!("ssh://{host}/{path}")
        };
        let url = url::Url::parse(&normalized).ok()?;
        if !matches!(url.scheme(), "ssh" | "https" | "http" | "git") {
            return None;
        }
        let host = url.host_str()?.to_ascii_lowercase();
        if !host
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'-')
        {
            return None;
        }
        let project = url
            .path()
            .trim_matches('/')
            .trim_end_matches(".git")
            .to_owned();
        if project.split('/').count() < 2
            || project
                .split('/')
                .any(|part| part.is_empty() || part == "..")
        {
            return None;
        }
        let provider = if host == "github.com"
            || host.ends_with(".ghe.com")
            || host.split('.').any(|part| part == "github")
        {
            Some(Provider::GitHub)
        } else if host == "gitlab.com" || host.split('.').any(|part| part == "gitlab") {
            Some(Provider::GitLab)
        } else {
            None
        };
        let host = match (url.scheme(), url.port()) {
            ("https" | "http", Some(port)) => format!("{host}:{port}"),
            _ => host,
        };
        Some(Self {
            host,
            project,
            provider,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Branch {
    pub name: String,
    pub reference: String,
    pub current: bool,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferenceKind {
    Local,
    Remote,
    Tag,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GraphRow {
    pub lane: usize,
    pub incoming: Vec<(usize, usize)>,
    pub outgoing: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub(crate) struct Commit {
    pub id: String,
    pub parents: Vec<String>,
    pub subject: String,
    pub author: String,
    pub date: String,
    pub refs: String,
    pub graph: GraphRow,
}

pub(crate) fn assign_graph(commits: &mut [Commit], mut lanes: Vec<String>) -> Vec<String> {
    for commit in commits {
        let previous = lanes.clone();
        let lane = lanes
            .iter()
            .position(|id| id == &commit.id)
            .unwrap_or_else(|| {
                lanes.push(commit.id.clone());
                lanes.len() - 1
            });
        let before = lanes.clone();
        lanes.remove(lane);
        for (index, parent) in commit.parents.iter().enumerate() {
            if !lanes.contains(parent) {
                lanes.insert((lane + index).min(lanes.len()), parent.clone());
            }
        }
        let incoming = previous
            .iter()
            .enumerate()
            .map(|(index, _)| (index, index))
            .collect();
        let mut outgoing = Vec::new();
        for (index, id) in before.iter().enumerate() {
            if index != lane
                && let Some(next) = lanes.iter().position(|target| target == id)
            {
                outgoing.push((index, next));
            }
        }
        for parent in &commit.parents {
            if let Some(next) = lanes.iter().position(|target| target == parent) {
                outgoing.push((lane, next));
            }
        }
        commit.graph = GraphRow {
            lane,
            incoming,
            outgoing,
        };
    }
    lanes
}

#[derive(Debug, Clone)]
pub(crate) struct HistoryCursor {
    pub revisions: Vec<String>,
    pub offset: usize,
    pub lanes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct HistoryPage {
    pub commits: Vec<Commit>,
    pub next: Option<HistoryCursor>,
}

#[derive(Debug, Clone)]
pub(crate) struct Repository {
    pub root: PathBuf,
    pub branches: Vec<Branch>,
    pub commits: Vec<Commit>,
    pub next_history: Option<HistoryCursor>,
    pub remote: Option<Remote>,
    pub head: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ChangedFile {
    pub path: String,
    pub status: char,
}

#[derive(Debug, Clone)]
pub(crate) struct Pipeline {
    pub id: u64,
    pub number: Option<u64>,
    pub title: String,
    pub branch: String,
    pub commit: Option<String>,
    pub status: String,
    pub date: String,
    pub url: String,
}

impl Pipeline {
    pub(crate) fn is_active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "running"
                | "in_progress"
                | "pending"
                | "queued"
                | "created"
                | "preparing"
                | "waiting_for_resource"
                | "waiting_for_callback"
                | "requested"
                | "waiting"
        )
    }
}

#[cfg(test)]
mod tests;
