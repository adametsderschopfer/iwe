use crate::app::{App, Message, Navigation, PipelineState};
#[cfg(test)]
use crate::domain::Failure;
use crate::domain::jobs::{Action, Details, JobAction};
use crate::i18n::Key;
pub(crate) use crate::modules::pipelines::jobs::*;
use iced::{Subscription, Task};
use std::time::Duration;
pub(crate) mod overview;

impl App {
    pub(in crate::app) fn reset_jobs(&mut self) {
        self.modules.pipelines.jobs = State {
            epoch: self.modules.pipelines.jobs.epoch + 1,
            ..State::default()
        };
    }

    pub(in crate::app) fn reload_jobs_after_sign_in(&mut self) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Pipelines) {
            return Task::none();
        }
        self.modules.pipelines.jobs.overview.clear();
        self.modules.pipelines.jobs.loading = false;
        self.modules.pipelines.jobs.error = None;
        self.modules.pipelines.jobs.action_error = None;
        self.modules.pipelines.jobs.request += 1;
        Task::batch([self.refresh_jobs(), self.refresh_pipeline_overviews(false)])
    }

    pub(in crate::app) fn jobs_subscription(&self) -> Subscription<Message> {
        if self.module_enabled(crate::modules::ModuleId::Pipelines)
            && self.navigation != Navigation::Settings
            && self.workspace.preferences.show_pipelines
            && (self.modules.pipelines.jobs.details.is_some()
                || !self.modules.pipelines.jobs.overview.entries.is_empty())
            && self.modules.pipelines.jobs.error.is_none()
            && self.modules.pipelines.jobs.action_error.is_none()
        {
            iced::time::every(Duration::from_secs(10)).map(|_| Message::Jobs(Event::Tick))
        } else {
            Subscription::none()
        }
    }

    pub(in crate::app) fn refresh_jobs(&mut self) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Pipelines) {
            return Task::none();
        }
        self.request_jobs(false)
    }

    fn request_jobs(&mut self, retry_credentials: bool) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Pipelines) {
            return Task::none();
        }
        if self.modules.pipelines.jobs.loading
            || self.disconnecting
            || self.modules.pipelines.jobs.pending.is_some()
        {
            return Task::none();
        }
        let Some(run) = self.modules.pipelines.jobs.selected.clone() else {
            return Task::none();
        };
        let Some(remote) = self
            .active_repository()
            .and_then(|repo| repo.remote.clone())
        else {
            return Task::none();
        };
        self.modules.pipelines.jobs.loading = true;
        self.modules.pipelines.jobs.error = None;
        self.modules.pipelines.jobs.request += 1;
        let (epoch, request, id) = (
            self.modules.pipelines.jobs.epoch,
            self.modules.pipelines.jobs.request,
            run.id,
        );
        let service = self.auth_service.clone();
        Task::perform(
            async move {
                if retry_credentials {
                    service.retry_credentials(&remote).await?;
                }
                service.load_jobs(remote, run).await
            },
            move |result| Message::Jobs(Event::Loaded(epoch, request, id, result)),
        )
    }

    pub(in crate::app) fn update_jobs(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Overview(event) => return self.update_pipeline_overview(event),
            Event::Select(id) => {
                if self
                    .modules
                    .pipelines
                    .jobs
                    .selected
                    .as_ref()
                    .is_some_and(|run| run.id == id)
                {
                    return Task::none();
                }
                let PipelineState::Ready(runs) = &self.modules.pipelines.pipelines else {
                    return Task::none();
                };
                let Some(run) = runs.iter().find(|run| run.id == id).cloned() else {
                    return Task::none();
                };
                self.modules.pipelines.jobs.selected = Some(run);
                self.modules.pipelines.jobs.overview.popup = None;
                self.modules.pipelines.jobs.overview.invalidate(id);
                self.modules.pipelines.jobs.details = self
                    .modules
                    .pipelines
                    .jobs
                    .overview
                    .entries
                    .get(&id)
                    .and_then(|entry| entry.details.clone());
                self.modules.pipelines.jobs.loading = false;
                self.modules.pipelines.jobs.error = None;
                self.modules.pipelines.jobs.action_error = None;
                self.modules.pipelines.jobs.collapsed.clear();
                self.modules.pipelines.jobs.request += 1;
                return self.refresh_jobs();
            }
            Event::Back => {
                self.modules.pipelines.jobs.selected = None;
                self.modules.pipelines.jobs.details = None;
                self.modules.pipelines.jobs.loading = false;
                self.modules.pipelines.jobs.error = None;
                self.modules.pipelines.jobs.request += 1;
                self.modules.pipelines.jobs.action_error = None;
                self.modules.pipelines.jobs.overview.popup = None;
                return self.refresh_pipeline_overviews(false);
            }
            Event::Refresh => {
                self.modules.pipelines.jobs.action_error = None;
                return self.request_jobs(true);
            }
            Event::Tick
                if self.modules.pipelines.jobs.error.is_none()
                    && self.modules.pipelines.jobs.action_error.is_none() =>
            {
                return if self.modules.pipelines.jobs.selected.is_some() {
                    self.refresh_jobs()
                } else {
                    self.refresh_pipeline_overviews(true)
                };
            }
            Event::ToggleStage(name) => {
                if !self.modules.pipelines.jobs.collapsed.remove(&name) {
                    self.modules.pipelines.jobs.collapsed.insert(name);
                }
            }
            Event::Loaded(epoch, request, id, result)
                if epoch == self.modules.pipelines.jobs.epoch
                    && request == self.modules.pipelines.jobs.request
                    && self
                        .modules
                        .pipelines
                        .jobs
                        .selected
                        .as_ref()
                        .is_some_and(|run| run.id == id) =>
            {
                self.modules.pipelines.jobs.loading = false;
                match result {
                    Ok(details) => {
                        if let Some(run) = &self.modules.pipelines.jobs.selected {
                            self.modules.pipelines.jobs.overview.remember(
                                id,
                                &run.status,
                                &details,
                            );
                        }
                        self.modules.pipelines.jobs.details = Some(details);
                        self.modules.pipelines.jobs.error = None;
                    }
                    Err(error) => self.modules.pipelines.jobs.error = Some(error),
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
                    .active_repository()
                    .and_then(|repo| repo.remote.clone())
                else {
                    return Task::none();
                };
                self.modules.pipelines.jobs.pending = Some((id, action));
                self.modules.pipelines.jobs.overview.invalidate(id);
                self.modules.pipelines.jobs.loading = false;
                self.modules.pipelines.jobs.action_error = None;
                self.modules.pipelines.jobs.request += 1;
                self.modules.pipelines.jobs.action_request += 1;
                let (epoch, request) = (
                    self.modules.pipelines.jobs.epoch,
                    self.modules.pipelines.jobs.action_request,
                );
                let service = self.auth_service.clone();
                return Task::perform(
                    async move { service.control_job(remote, id, action).await },
                    move |result| Message::Jobs(Event::Controlled(epoch, request, id, result)),
                );
            }
            Event::Controlled(epoch, request, id, result)
                if epoch == self.modules.pipelines.jobs.epoch
                    && request == self.modules.pipelines.jobs.action_request
                    && self
                        .modules
                        .pipelines
                        .jobs
                        .pending
                        .is_some_and(|(pipeline, _)| pipeline == id) =>
            {
                let action = self
                    .modules
                    .pipelines
                    .jobs
                    .pending
                    .take()
                    .map(|(_, action)| action);
                let context = action.and_then(|action| match action {
                    Action::Job(job, _) => self
                        .job_details(id)
                        .and_then(|details| {
                            details
                                .stages
                                .iter()
                                .flat_map(|stage| &stage.jobs)
                                .find(|item| item.id == job)
                        })
                        .map(|job| job.name.clone()),
                    Action::CancelPipeline => self
                        .modules
                        .pipelines
                        .jobs
                        .selected
                        .as_ref()
                        .filter(|run| run.id == id)
                        .map(|run| run.title.clone()),
                });
                let selected = self
                    .modules
                    .pipelines
                    .jobs
                    .selected
                    .as_ref()
                    .is_some_and(|run| run.id == id);
                match result {
                    Ok(()) => {
                        if let Some(action) = action {
                            self.notifications.success(
                                match action {
                                    Action::Job(_, JobAction::Play) => Key::JobRunRequested,
                                    Action::Job(_, JobAction::Retry) => Key::JobRetryRequested,
                                    Action::Job(_, JobAction::Cancel) => Key::JobStopRequested,
                                    Action::CancelPipeline => Key::PipelineStopRequested,
                                },
                                context,
                            );
                        }
                        return Task::batch([
                            self.load_pipelines(),
                            self.refresh_jobs(),
                            self.refresh_pipeline_overviews(false),
                        ]);
                    }
                    Err(error) => {
                        self.notifications
                            .error(crate::app::error_key(error), context);
                        if selected {
                            self.modules.pipelines.jobs.action_error = Some(error);
                            return Task::none();
                        }
                        self.modules.pipelines.jobs.overview.failed(id, error);
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
        if self
            .modules
            .pipelines
            .jobs
            .selected
            .as_ref()
            .is_some_and(|run| run.id == id)
        {
            self.modules.pipelines.jobs.details.as_ref()
        } else if self.modules.pipelines.jobs.selected.is_none()
            && matches!(&self.modules.pipelines.pipelines, PipelineState::Ready(runs) if runs.iter().any(|run| run.id == id))
        {
            self.modules
                .pipelines
                .jobs
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
            || self.modules.pipelines.jobs.pending.is_some()
            || if self
                .modules
                .pipelines
                .jobs
                .selected
                .as_ref()
                .is_some_and(|run| run.id == id)
            {
                self.modules.pipelines.jobs.error.is_some()
                    || self.modules.pipelines.jobs.action_error.is_some()
            } else {
                self.modules
                    .pipelines
                    .jobs
                    .overview
                    .entries
                    .get(&id)
                    .is_none_or(|entry| entry.error.is_some())
            }
    }
}

#[cfg(test)]
mod tests;
