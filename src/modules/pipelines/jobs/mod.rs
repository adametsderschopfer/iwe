//! Keep pipeline details visible during refresh and isolate provider actions by project.

use crate::domain::{
    Failure,
    jobs::{Action, Details},
    repository::Pipeline,
};

use std::collections::BTreeSet;

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
    pub(crate) epoch: u64,
    pub(crate) request: u64,
    pub(crate) action_request: u64,
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
