//! Cache visible pipeline summaries without preloading every run's jobs.

use super::{App, Details, Event as JobsEvent, Failure, Message, Navigation, PipelineState};
use iced::{Point, Task};
use std::{collections::BTreeMap, ops::Range};

const MAX_LOADING: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Stage(String),
    Job(u64),
}

impl Target {
    pub(crate) fn exists(&self, details: &Details) -> bool {
        match self {
            Self::Stage(name) => details
                .stages
                .iter()
                .any(|stage| stage.name.as_ref() == Some(name)),
            Self::Job(id) => details
                .stages
                .iter()
                .flat_map(|stage| &stage.jobs)
                .any(|job| job.id == *id),
        }
    }
}

pub(crate) struct Popup {
    pub pipeline: u64,
    pub target: Target,
    pub position: Point,
}

#[derive(Default)]
pub(crate) struct Entry {
    pub details: Option<Details>,
    pub error: Option<Failure>,
    loading: Option<u64>,
    status: String,
    stale: bool,
}

impl Entry {
    pub(crate) fn is_loading(&self) -> bool {
        self.loading.is_some()
    }
}

pub(crate) struct State {
    pub entries: BTreeMap<u64, Entry>,
    pub popup: Option<Popup>,
    visible: Range<usize>,
    request: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            popup: None,
            visible: 0..30,
            request: 0,
        }
    }
}

#[cfg(test)]
mod tests;

impl State {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.popup = None;
    }

    pub(super) fn remember(&mut self, id: u64, status: &str, details: &Details) {
        let entry = self.entries.entry(id).or_default();
        entry.details = Some(details.clone());
        entry.error = None;
        entry.loading = None;
        entry.stale = false;
        entry.status = status.to_owned();
    }

    pub(super) fn invalidate(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.loading = None;
            entry.stale = true;
            entry.error = None;
        }
    }

    pub(super) fn failed(&mut self, id: u64, error: Failure) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.error = Some(error);
            entry.loading = None;
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Viewport(Range<usize>),
    Open(u64, Target, Point),
    Close,
    Refresh(u64),
    RefreshVisible,
    Loaded(u64, u64, u64, Result<Details, Failure>),
}

impl App {
    pub(in crate::app) fn refresh_pipeline_overviews(&mut self, active: bool) -> Task<Message> {
        if self.navigation == Navigation::Settings
            || !self.workspace.preferences.show_pipelines
            || self.jobs.selected.is_some()
            || self.disconnecting
            || self.jobs.pending.is_some()
        {
            return Task::none();
        }
        let PipelineState::Ready(runs) = &self.pipelines else {
            return Task::none();
        };
        let Some(remote) = self
            .repository
            .as_ref()
            .and_then(|repo| repo.remote.clone())
        else {
            return Task::none();
        };
        let state = &mut self.jobs.overview;
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
            let (epoch, id) = (self.jobs.epoch, run.id);
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

    pub(super) fn update_pipeline_overview(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Viewport(range) => {
                self.jobs.overview.visible = range;
                return self.refresh_pipeline_overviews(false);
            }
            Event::Open(pipeline, target, position) => {
                let state = &mut self.jobs.overview;
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
            Event::Close => self.jobs.overview.popup = None,
            Event::Refresh(id) => {
                if self.jobs.pending.is_some()
                    || self
                        .jobs
                        .overview
                        .entries
                        .get(&id)
                        .is_some_and(Entry::is_loading)
                {
                    return Task::none();
                }
                self.jobs.overview.invalidate(id);
                return self.refresh_pipeline_overviews(false);
            }
            Event::RefreshVisible => {
                if self.jobs.pending.is_some() {
                    return Task::none();
                }
                for entry in self.jobs.overview.entries.values_mut() {
                    if entry.loading.is_none() {
                        entry.error = None;
                        entry.stale = true;
                    }
                }
                self.jobs.action_error = None;
                // Retry the shared account before starting summary requests.
                return self.request_pipelines(true).chain(Task::batch([
                    self.refresh_jobs(),
                    self.refresh_pipeline_overviews(false),
                ]));
            }
            Event::Loaded(epoch, request, id, result) if epoch == self.jobs.epoch => {
                let state = &mut self.jobs.overview;
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
