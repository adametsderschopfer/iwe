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
    pub provider_status: Option<Status>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Passed,
    Failed,
    Running,
    Canceling,
    Pending,
    Manual,
    Cancelled,
    Skipped,
    Unknown,
}

impl Status {
    pub(crate) fn from_provider(value: &str) -> Self {
        match value {
            "success" | "passed" => Self::Passed,
            "failure" | "failed" | "timed_out" | "startup_failure" | "action_required" => {
                Self::Failed
            }
            "running" | "in_progress" => Self::Running,
            "canceling" => Self::Canceling,
            "queued"
            | "pending"
            | "waiting"
            | "waiting_for_resource"
            | "waiting_for_callback"
            | "requested"
            | "created"
            | "preparing"
            | "scheduled" => Self::Pending,
            "manual" => Self::Manual,
            "cancelled" | "canceled" => Self::Cancelled,
            "skipped" | "neutral" => Self::Skipped,
            _ => Self::Unknown,
        }
    }
}

impl Stage {
    pub(crate) fn status(&self) -> Status {
        if let Some(status) = self.provider_status {
            return status;
        }
        // Never report success while a job is unfinished or its status is unknown.
        self.jobs
            .iter()
            .map(|job| Status::from_provider(&job.status))
            .min_by_key(|status| match status {
                Status::Failed => 0,
                Status::Canceling => 1,
                Status::Running => 2,
                Status::Pending => 3,
                Status::Manual => 4,
                Status::Cancelled => 5,
                Status::Unknown => 6,
                Status::Passed => 7,
                Status::Skipped => 8,
            })
            .unwrap_or(Status::Skipped)
    }
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
