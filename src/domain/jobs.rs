//! Provider job capabilities and stage ordering, independent of the interface.

use super::repository::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobAction {
    Play,
    Retry,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Job(u64, JobAction),
    CancelPipeline,
}

#[derive(Debug, Clone)]
pub(crate) struct Job {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub url: String,
    pub actions: Vec<JobAction>,
}

#[derive(Debug, Clone)]
pub(crate) struct Stage {
    // GitHub has no stage entity. Its jobs use an unnamed group.
    pub name: Option<String>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Details {
    pub stages: Vec<Stage>,
    pub cancelable: bool,
}

impl Details {
    pub(crate) fn allows(&self, action: Action) -> bool {
        match action {
            Action::CancelPipeline => self.cancelable,
            Action::Job(id, action) => self
                .stages
                .iter()
                .flat_map(|stage| &stage.jobs)
                .any(|job| job.id == id && job.actions.contains(&action)),
        }
    }
}

impl JobAction {
    pub(crate) fn supported(self, provider: Provider) -> bool {
        provider == Provider::GitLab || self == Self::Retry
    }
}

#[cfg(test)]
mod tests;
