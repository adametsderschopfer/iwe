//! Keep pipeline details visible during refresh and isolate provider actions by project.

use super::{App, Message, Navigation, PipelineState};
use crate::domain::{
    Failure,
    jobs::{Action, Details},
    repository::Pipeline,
};
use iced::{Subscription, Task};
use std::{collections::BTreeSet, time::Duration};

pub(crate) mod overview;

#[derive(Default)]
pub(crate) struct State {
    pub selected: Option<Pipeline>,
    pub details: Option<Details>,
    pub loading: bool,
    pub error: Option<Failure>,
    pub action_error: Option<Failure>,
    pub pending: Option<(u64, Action)>,
    pub collapsed: BTreeSet<String>,
    pub overview: overview::State,
    epoch: u64,
    request: u64,
    action_request: u64,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Select(u64),
    Back,
    Refresh,
    Tick,
    ToggleStage(String),
    Loaded(u64, u64, u64, Result<Details, Failure>),
    Control(u64, Action),
    Controlled(u64, u64, u64, Result<(), Failure>),
    Overview(overview::Event),
}

impl App {
    pub(super) fn reset_jobs(&mut self) {
        self.jobs = State {
            epoch: self.jobs.epoch + 1,
            ..State::default()
        };
    }

    pub(super) fn reload_jobs_after_sign_in(&mut self) -> Task<Message> {
        self.jobs.overview.clear();
        self.jobs.loading = false;
        self.jobs.error = None;
        self.jobs.action_error = None;
        self.jobs.request += 1;
        Task::batch([self.refresh_jobs(), self.refresh_pipeline_overviews(false)])
    }

    pub(super) fn jobs_subscription(&self) -> Subscription<Message> {
        if self.navigation != Navigation::Settings
            && self.workspace.preferences.show_pipelines
            && (self.jobs.details.is_some() || !self.jobs.overview.entries.is_empty())
            && self.jobs.error.is_none()
            && self.jobs.action_error.is_none()
        {
            iced::time::every(Duration::from_secs(10)).map(|_| Message::Jobs(Event::Tick))
        } else {
            Subscription::none()
        }
    }

    pub(super) fn refresh_jobs(&mut self) -> Task<Message> {
        if self.jobs.loading || self.disconnecting || self.jobs.pending.is_some() {
            return Task::none();
        }
        let Some(run) = self.jobs.selected.clone() else {
            return Task::none();
        };
        let Some(remote) = self
            .repository
            .as_ref()
            .and_then(|repo| repo.remote.clone())
        else {
            return Task::none();
        };
        self.jobs.loading = true;
        self.jobs.error = None;
        self.jobs.request += 1;
        let (epoch, request, id) = (self.jobs.epoch, self.jobs.request, run.id);
        let service = self.auth_service.clone();
        Task::perform(
            async move { service.load_jobs(remote, run).await },
            move |result| Message::Jobs(Event::Loaded(epoch, request, id, result)),
        )
    }

    pub(super) fn update_jobs(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Overview(event) => return self.update_pipeline_overview(event),
            Event::Select(id) => {
                if self.jobs.selected.as_ref().is_some_and(|run| run.id == id) {
                    return Task::none();
                }
                let PipelineState::Ready(runs) = &self.pipelines else {
                    return Task::none();
                };
                let Some(run) = runs.iter().find(|run| run.id == id).cloned() else {
                    return Task::none();
                };
                self.jobs.selected = Some(run);
                self.jobs.overview.popup = None;
                self.jobs.overview.invalidate(id);
                self.jobs.details = self
                    .jobs
                    .overview
                    .entries
                    .get(&id)
                    .and_then(|entry| entry.details.clone());
                self.jobs.loading = false;
                self.jobs.error = None;
                self.jobs.action_error = None;
                self.jobs.collapsed.clear();
                self.jobs.request += 1;
                return self.refresh_jobs();
            }
            Event::Back => {
                self.jobs.selected = None;
                self.jobs.details = None;
                self.jobs.loading = false;
                self.jobs.error = None;
                self.jobs.request += 1;
                self.jobs.action_error = None;
                self.jobs.overview.popup = None;
                return self.refresh_pipeline_overviews(false);
            }
            Event::Refresh => {
                self.jobs.action_error = None;
                return self.refresh_jobs();
            }
            Event::Tick if self.jobs.error.is_none() && self.jobs.action_error.is_none() => {
                return if self.jobs.selected.is_some() {
                    self.refresh_jobs()
                } else {
                    self.refresh_pipeline_overviews(true)
                };
            }
            Event::ToggleStage(name) => {
                if !self.jobs.collapsed.remove(&name) {
                    self.jobs.collapsed.insert(name);
                }
            }
            Event::Loaded(epoch, request, id, result)
                if epoch == self.jobs.epoch
                    && request == self.jobs.request
                    && self.jobs.selected.as_ref().is_some_and(|run| run.id == id) =>
            {
                self.jobs.loading = false;
                match result {
                    Ok(details) => {
                        if let Some(run) = &self.jobs.selected {
                            self.jobs.overview.remember(id, &run.status, &details);
                        }
                        self.jobs.details = Some(details);
                        self.jobs.error = None;
                    }
                    Err(error) => self.jobs.error = Some(error),
                }
            }
            Event::Control(id, action) => {
                if self.job_controls_blocked(id)
                    || !self
                        .job_details(id)
                        .is_some_and(|details| details.allows(action))
                {
                    return Task::none();
                }
                let Some(remote) = self
                    .repository
                    .as_ref()
                    .and_then(|repo| repo.remote.clone())
                else {
                    return Task::none();
                };
                self.jobs.pending = Some((id, action));
                self.jobs.overview.invalidate(id);
                self.jobs.loading = false;
                self.jobs.action_error = None;
                self.jobs.request += 1;
                self.jobs.action_request += 1;
                let (epoch, request) = (self.jobs.epoch, self.jobs.action_request);
                let service = self.auth_service.clone();
                return Task::perform(
                    async move { service.control_job(remote, id, action).await },
                    move |result| Message::Jobs(Event::Controlled(epoch, request, id, result)),
                );
            }
            Event::Controlled(epoch, request, id, result)
                if epoch == self.jobs.epoch
                    && request == self.jobs.action_request
                    && self
                        .jobs
                        .pending
                        .is_some_and(|(pipeline, _)| pipeline == id) =>
            {
                self.jobs.pending = None;
                let selected = self.jobs.selected.as_ref().is_some_and(|run| run.id == id);
                match result {
                    Ok(()) => {
                        return Task::batch([
                            self.load_pipelines(),
                            self.refresh_jobs(),
                            self.refresh_pipeline_overviews(false),
                        ]);
                    }
                    Err(error) if selected => self.jobs.action_error = Some(error),
                    Err(error) => {
                        self.jobs.overview.failed(id, error);
                        self.notice = Some(super::error_key(error));
                        return Task::batch([
                            self.refresh_jobs(),
                            self.refresh_pipeline_overviews(false),
                        ]);
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn job_details(&self, id: u64) -> Option<&Details> {
        if self.jobs.selected.as_ref().is_some_and(|run| run.id == id) {
            self.jobs.details.as_ref()
        } else if self.jobs.selected.is_none()
            && matches!(&self.pipelines, PipelineState::Ready(runs) if runs.iter().any(|run| run.id == id))
        {
            self.jobs
                .overview
                .entries
                .get(&id)
                .and_then(|entry| entry.details.as_ref())
        } else {
            None
        }
    }

    pub(crate) fn job_controls_blocked(&self, id: u64) -> bool {
        self.disconnecting
            || self.jobs.pending.is_some()
            || if self.jobs.selected.as_ref().is_some_and(|run| run.id == id) {
                self.jobs.error.is_some() || self.jobs.action_error.is_some()
            } else {
                self.jobs
                    .overview
                    .entries
                    .get(&id)
                    .is_none_or(|entry| entry.error.is_some())
            }
    }
}

#[cfg(test)]
mod tests;
