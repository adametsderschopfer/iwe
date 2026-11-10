//! Cache visible pipeline summaries without preloading every run's jobs.

use crate::domain::{Failure, jobs::Details};
use iced::Point;
use std::{collections::BTreeMap, ops::Range};

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
    pub(crate) loading: Option<u64>,
    pub(crate) status: String,
    pub(crate) stale: bool,
}

impl Entry {
    pub(crate) fn is_loading(&self) -> bool {
        self.loading.is_some()
    }
}

pub(crate) struct State {
    pub entries: BTreeMap<u64, Entry>,
    pub popup: Option<Popup>,
    pub(crate) visible: Range<usize>,
    pub(crate) request: u64,
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

impl State {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.popup = None;
    }

    pub(crate) fn remember(&mut self, id: u64, status: &str, details: &Details) {
        let entry = self.entries.entry(id).or_default();
        entry.details = Some(details.clone());
        entry.error = None;
        entry.loading = None;
        entry.stale = false;
        entry.status = status.to_owned();
    }

    pub(crate) fn invalidate(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.loading = None;
            entry.stale = true;
            entry.error = None;
        }
    }

    pub(crate) fn failed(&mut self, id: u64, error: Failure) {
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
