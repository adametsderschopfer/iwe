//! Bundled pipelines state and public integration boundary.

use crate::domain::{Failure, repository::Pipeline};
use cache as pipeline_cache;
mod cache;
pub(crate) mod jobs;
pub(crate) mod transport;

#[derive(Debug, Clone, Default)]
pub(crate) enum PipelineState {
    #[default]
    Idle,
    Loading,
    Ready(Vec<Pipeline>),
    Failed(Failure),
}

pub(crate) struct State {
    pub pipelines: PipelineState,
    pub jobs: jobs::State,
    pub pipeline_request: u64,
    pub pipeline_refreshing: bool,
    pub pipeline_cache: pipeline_cache::Cache,
    pub pipeline_poll_blocked: bool,
    pub pipeline_reload_after_repository: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pipelines: PipelineState::Idle,
            jobs: jobs::State::default(),
            pipeline_request: 0,
            pipeline_refreshing: false,
            pipeline_cache: pipeline_cache::Cache::default(),
            pipeline_poll_blocked: false,
            pipeline_reload_after_repository: false,
        }
    }
}

mod view;
pub(crate) use view::{Event, View, overlay, view};
