use super::Event as JobsEvent;
use super::{App, Message, Navigation, PipelineState};
#[cfg(test)]
use crate::domain::Failure;
pub(crate) use crate::modules::pipelines::jobs::overview::*;
use iced::Task;
const MAX_LOADING: usize = 2;

impl App {
    pub(in crate::app) fn refresh_pipeline_overviews(&mut self, active: bool) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Pipelines) {
            return Task::none();
        }
        if self.navigation == Navigation::Settings
            || !self.workspace.preferences.show_pipelines
            || self.modules.pipelines.jobs.selected.is_some()
            || self.disconnecting
            || self.modules.pipelines.jobs.pending.is_some()
        {
            return Task::none();
        }
        let PipelineState::Ready(runs) = &self.modules.pipelines.pipelines else {
            return Task::none();
        };
        let Some(remote) = self
            .active_repository()
            .and_then(|repo| repo.remote.clone())
        else {
            return Task::none();
        };
        let state = &mut self.modules.pipelines.jobs.overview;
        state
            .entries
            .retain(|id, _| runs.iter().any(|run| run.id == *id));
        if state
            .popup
            .as_ref()
            .is_some_and(|popup| !state.entries.contains_key(&popup.pipeline))
        {
            state.popup = None;
        }
        let mut visible = runs
            .iter()
            .enumerate()
            .filter(|(index, run)| {
                state.visible.contains(index)
                    || state
                        .popup
                        .as_ref()
                        .is_some_and(|popup| popup.pipeline == run.id)
            })
            .map(|(_, run)| run)
            .collect::<Vec<_>>();
        if let Some(popup) = &state.popup {
            visible.sort_by_key(|run| run.id != popup.pipeline);
        }
        for run in &visible {
            let entry = state.entries.entry(run.id).or_default();
            if active && run.is_active() && entry.loading.is_none() {
                entry.stale = true;
            }
        }
        let mut available = MAX_LOADING.saturating_sub(
            state
                .entries
                .values()
                .filter(|entry| entry.loading.is_some())
                .count(),
        );
        let mut tasks = Vec::new();
        for run in visible {
            if available == 0 {
                break;
            }
            let entry = state.entries.entry(run.id).or_default();
            if entry.loading.is_some()
                || entry.error.is_some()
                || (entry.details.is_some() && !entry.stale && entry.status == run.status)
            {
                continue;
            }
            available -= 1;
            state.request += 1;
            let request = state.request;
            entry.loading = Some(request);
            entry.status.clone_from(&run.status);
            entry.stale = false;
            let (epoch, id) = (self.modules.pipelines.jobs.epoch, run.id);
            let service = self.auth_service.clone();
            let remote = remote.clone();
            let run = run.clone();
            tasks.push(Task::perform(
                async move { service.load_jobs(remote, run).await },
                move |result| {
                    Message::Jobs(JobsEvent::Overview(Event::Loaded(
                        epoch, request, id, result,
                    )))
                },
            ));
        }
        Task::batch(tasks)
    }

    pub(in crate::app) fn update_pipeline_overview(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Viewport(range) => {
                self.modules.pipelines.jobs.overview.visible = range;
                return self.refresh_pipeline_overviews(false);
            }
            Event::Open(pipeline, target, position) => {
                let state = &mut self.modules.pipelines.jobs.overview;
                if state
                    .entries
                    .get(&pipeline)
                    .and_then(|entry| entry.details.as_ref())
                    .is_none_or(|details| !target.exists(details))
                {
                    return Task::none();
                }
                state.popup = if state
                    .popup
                    .as_ref()
                    .is_some_and(|popup| popup.pipeline == pipeline && popup.target == target)
                {
                    None
                } else {
                    Some(Popup {
                        pipeline,
                        target,
                        position,
                    })
                };
            }
            Event::Close => self.modules.pipelines.jobs.overview.popup = None,
            Event::Refresh(id) => {
                if self.modules.pipelines.jobs.pending.is_some()
                    || self
                        .modules
                        .pipelines
                        .jobs
                        .overview
                        .entries
                        .get(&id)
                        .is_some_and(Entry::is_loading)
                {
                    return Task::none();
                }
                self.modules.pipelines.jobs.overview.invalidate(id);
                return self.refresh_pipeline_overviews(false);
            }
            Event::RefreshVisible => {
                if self.modules.pipelines.jobs.pending.is_some() {
                    return Task::none();
                }
                for entry in self.modules.pipelines.jobs.overview.entries.values_mut() {
                    if entry.loading.is_none() {
                        entry.error = None;
                        entry.stale = true;
                    }
                }
                self.modules.pipelines.jobs.action_error = None;
                // Retry the shared account before starting summary requests.
                return self.request_pipelines(true).chain(Task::batch([
                    self.refresh_jobs(),
                    self.refresh_pipeline_overviews(false),
                ]));
            }
            Event::Loaded(epoch, request, id, result)
                if epoch == self.modules.pipelines.jobs.epoch =>
            {
                let state = &mut self.modules.pipelines.jobs.overview;
                let Some(entry) = state
                    .entries
                    .get_mut(&id)
                    .filter(|entry| entry.loading == Some(request))
                else {
                    return Task::none();
                };
                entry.loading = None;
                match result {
                    Ok(details) => {
                        if state.popup.as_ref().is_some_and(|popup| {
                            popup.pipeline == id && !popup.target.exists(&details)
                        }) {
                            state.popup = None;
                        }
                        entry.details = Some(details);
                        entry.error = None;
                    }
                    Err(error) => entry.error = Some(error),
                }
                return self.refresh_pipeline_overviews(false);
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
