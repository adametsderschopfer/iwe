//! Keep pipeline details visible during refresh and isolate provider actions by project.

use super::{App, Message, Navigation, PipelineState};
use crate::domain::{
    Failure,
    jobs::{Action, Details},
    repository::Pipeline,
};
use iced::{Subscription, Task};
use std::{collections::BTreeSet, time::Duration};

#[derive(Default)]
pub(crate) struct State {
    pub selected: Option<Pipeline>,
    pub details: Option<Details>,
    pub loading: bool,
    pub error: Option<Failure>,
    pub action_error: Option<Failure>,
    pub pending: Option<(u64, Action)>,
    pub collapsed: BTreeSet<String>,
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
}

impl App {
    pub(super) fn reset_jobs(&mut self) {
        self.jobs = State {
            epoch: self.jobs.epoch + 1,
            ..State::default()
        };
    }

    pub(super) fn reload_jobs_after_sign_in(&mut self) -> Task<Message> {
        self.jobs.loading = false;
        self.jobs.error = None;
        self.jobs.action_error = None;
        self.jobs.request += 1;
        self.refresh_jobs()
    }

    pub(super) fn jobs_subscription(&self) -> Subscription<Message> {
        if self.navigation != Navigation::Settings
            && self.workspace.preferences.show_pipelines
            && self.jobs.details.is_some()
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
                self.jobs.details = None;
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
            }
            Event::Refresh => {
                self.jobs.action_error = None;
                return self.refresh_jobs();
            }
            Event::Tick if self.jobs.error.is_none() && self.jobs.action_error.is_none() => {
                return self.refresh_jobs();
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
                        self.jobs.details = Some(details);
                        self.jobs.error = None;
                    }
                    Err(error) => self.jobs.error = Some(error),
                }
            }
            Event::Control(id, action) => {
                if self.disconnecting
                    || self.jobs.pending.is_some()
                    || self.jobs.error.is_some()
                    || self.jobs.action_error.is_some()
                    || self.jobs.selected.as_ref().is_none_or(|run| run.id != id)
                    || !self
                        .jobs
                        .details
                        .as_ref()
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
                    Ok(()) => return Task::batch([self.load_pipelines(), self.refresh_jobs()]),
                    Err(error) if selected => self.jobs.action_error = Some(error),
                    Err(error) => {
                        self.notice = Some(super::error_key(error));
                        return self.refresh_jobs();
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
