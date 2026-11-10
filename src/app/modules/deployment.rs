//! Connect deployment results to workspace persistence and source tabs.

use super::deployment_message;
use crate::{
    app::{App, Message, Navigation, SettingsTab},
    domain::deployment::Profile,
    modules::deployment::{Effect, Event, Operation, Output},
};
use iced::Task;

impl App {
    pub(crate) fn deployment_profiles(&self) -> &[Profile] {
        self.workspace
            .selected_space
            .and_then(|index| self.workspace.spaces.get(index))
            .map(|space| space.deployments.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn update_deployment(&mut self, mut event: Event) -> Task<Message> {
        if let Event::SpaceSettings(index) = event {
            let Some(space) = self.workspace.spaces.get(index) else {
                return Task::none();
            };
            if self.workspace.selected_space != Some(index) {
                self.workspace.selected_project = (!space.projects.is_empty()).then_some(0);
                self.workspace.selected_space = Some(index);
            }
            self.space_menu = None;
            self.settings_tab = SettingsTab::Deployment;
            return Task::batch([self.load_directory(), self.save(), self.open_settings()]);
        }
        let project = self.workspace.directory().cloned();
        self.modules
            .deployment
            .sync_context(self.workspace.selected_space, project.as_deref());
        if let Event::Resize(width, finished) = event {
            if width.is_finite() {
                self.workspace.preferences.deployment_panel_width = width.clamp(220., 640.);
            }
            return if finished { self.save() } else { Task::none() };
        }
        if let Event::CurrentFile(compare) = event {
            if self.navigation == Navigation::Settings
                || self.auth_dialog.is_some()
                || self.space_draft.is_some()
                || self.project_add.is_some()
                || self.line_navigation.is_some()
                || self.git_actions.has_dialog()
            {
                return Task::none();
            }
            let remote_selection = self
                .modules
                .deployment
                .menu
                .as_ref()
                .map(|entry| entry.path.clone());
            let path = if let Some(remote) = remote_selection {
                let path = self
                    .modules
                    .deployment
                    .active_profile(self.deployment_profiles(), project.as_deref())
                    .and_then(|profile| profile.to_local(&remote).ok());
                if path.is_none() {
                    self.modules.deployment.error = Some(crate::domain::deployment::Error::Mapping);
                    self.settings_tab = SettingsTab::Deployment;
                    return self.open_settings();
                }
                path
            } else {
                self.active_local_file()
            };
            let Some(path) = path else {
                return Task::none();
            };
            event = Event::Run(if compare {
                Operation::Compare(path)
            } else {
                Operation::Upload(path)
            });
        }
        self.files.menu = None;
        if matches!(event, Event::DeleteConnection)
            && self
                .modules
                .deployment
                .connection
                .as_ref()
                .is_some_and(|connection| {
                    self.workspace.spaces.iter().any(|space| {
                        space
                            .deployments
                            .iter()
                            .any(|profile| profile.connection == connection.id)
                    })
                })
        {
            self.modules.deployment.error = Some(crate::domain::deployment::Error::Configuration);
            return Task::none();
        }
        let mut empty = Vec::new();
        let profiles = self
            .workspace
            .selected_space
            .and_then(|index| self.workspace.spaces.get_mut(index))
            .map(|space| &mut space.deployments)
            .unwrap_or(&mut empty);
        let (task, effect) = self.modules.deployment.update(
            event,
            &mut self.workspace.deployment,
            profiles,
            project.as_deref(),
        );
        let task = task.map(deployment_message);
        match effect {
            Effect::None => task,
            Effect::Transferred(name) => {
                self.notifications
                    .success(crate::i18n::Key::DeploymentComplete, Some(name));
                task
            }
            Effect::Save => Task::batch([task, self.save()]),
            Effect::Settings => {
                self.settings_tab = SettingsTab::Deployment;
                Task::batch([task, self.open_settings()])
            }
            Effect::Preview(Output::Preview {
                profile_id,
                label,
                path,
                remote,
                text,
                previous,
            }) => Task::batch([
                task,
                self.open_remote_preview(path, remote, (profile_id, label), text, previous),
            ]),
            Effect::Preview(_) => task,
        }
    }
}
