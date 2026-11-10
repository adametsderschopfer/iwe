pub(crate) mod jobs;

use crate::app::{App, Message, Navigation, PipelineState};
#[cfg(test)]
use crate::domain::repository::Pipeline;
use crate::modules::{ModuleId, pipelines::transport as pipelines};
use iced::{Subscription, Task};
impl App {
    pub(in crate::app) fn load_pipelines(&mut self) -> Task<Message> {
        self.request_pipelines(false)
    }

    pub(in crate::app) fn request_pipelines(&mut self, retry_credentials: bool) -> Task<Message> {
        if !self.module_enabled(ModuleId::Pipelines)
            || self.disconnecting
            || self.modules.pipelines.pipeline_refreshing
        {
            return Task::none();
        }
        let Some(remote) = self
            .active_repository()
            .and_then(|repo| repo.remote.clone())
            .filter(|remote| remote.provider.is_some())
        else {
            return Task::none();
        };
        self.modules.pipelines.pipeline_request += 1;
        self.modules.pipelines.pipeline_refreshing = true;
        if !matches!(self.modules.pipelines.pipelines, PipelineState::Ready(_))
            && let Some(root) = self.active_repository().map(|repo| repo.root.clone())
            && let Some(runs) = self.modules.pipelines.pipeline_cache.get(&root, &remote)
        {
            self.modules.pipelines.pipelines = PipelineState::Ready(runs);
        }
        if !matches!(self.modules.pipelines.pipelines, PipelineState::Ready(_)) {
            self.modules.pipelines.pipelines = PipelineState::Loading;
        }
        let (generation, request) = (self.generation, self.modules.pipelines.pipeline_request);
        let service = self.auth_service.clone();
        Task::perform(
            async move {
                if retry_credentials && let Err(error) = service.retry_credentials(&remote).await {
                    return pipelines::Loaded {
                        runs: Err(error),
                        username: None,
                    };
                }
                service.load(remote).await
            },
            move |result| Message::PipelinesLoaded(generation, request, result),
        )
    }

    pub(in crate::app) fn pipeline_subscription(&self) -> Subscription<Message> {
        if self.module_enabled(ModuleId::Pipelines)
            && self.navigation != Navigation::Settings
            && !self.modules.pipelines.pipeline_poll_blocked
            && self.has_remote()
            && (self.workspace.preferences.show_git || self.workspace.preferences.show_pipelines)
            && matches!(self.modules.pipelines.pipelines, PipelineState::Ready(_))
        {
            iced::time::every(std::time::Duration::from_secs(30)).map(|_| Message::PollPipelines)
        } else {
            Subscription::none()
        }
    }

    #[cfg(test)]
    pub(crate) fn active_pipelines(&self, commit: &str) -> Vec<&Pipeline> {
        match &self.modules.pipelines.pipelines {
            PipelineState::Ready(runs) => runs
                .iter()
                .filter(|run| run.commit.as_deref() == Some(commit) && run.is_active())
                .collect(),
            _ => Vec::new(),
        }
    }
}
