//! Connect deployment results to workspace persistence and source tabs.

use super::deployment_message;
use crate::{
    app::{App, Message, Navigation, SettingsTab},
    domain::deployment::Profile,
    modules::deployment::{Effect, Event, NameAction, Operation, Output},
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

    pub(crate) fn deployment_local_path(&self, message: &Message) -> Option<std::path::PathBuf> {
        let Message::Module(super::Event::Deployment(event)) = message else {
            return None;
        };
        let operation = match event {
            Event::CurrentFile(_) => return self.active_local_file(),
            Event::Run(operation) | Event::RunOnProfile(_, operation) => operation,
            Event::ChooseTarget(_) => &self.modules.deployment.target.as_ref()?.0,
            Event::RemoteCompare(path) | Event::RemoteUpload(path) => {
                return self
                    .modules
                    .deployment
                    .active_profile(
                        self.deployment_profiles(),
                        self.workspace.directory().map(|p| p.as_path()),
                    )?
                    .to_local(path)
                    .ok();
            }
            _ => return None,
        };
        match operation {
            Operation::Upload(path) | Operation::Compare(path) => Some(path.clone()),
            _ => None,
        }
    }

    pub(super) fn update_deployment(&mut self, mut event: Event) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Deployment) {
            return Task::none();
        }
        if let Event::SpaceSettings(index) = event {
            let Some(space) = self.workspace.spaces.get(index) else {
                return Task::none();
            };
            if self.workspace.selected_space != Some(index) {
                self.workspace.selected_project = (!space.projects.is_empty()).then_some(0);
                self.workspace.selected_space = Some(index);
            }
            self.space_menu = None;
            return Task::batch([
                self.load_directory(),
                self.save(),
                self.update(Message::SelectSettingsTab(SettingsTab::Deployment)),
            ]);
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
                || self.modules.git.git_actions.has_dialog()
            {
                return Task::none();
            }
            let path = self.active_local_file();
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
        let duplicate = matches!(event, Event::ConfirmName)
            && self.modules.deployment.naming == Some(NameAction::DuplicateProfile);
        let saving = matches!(event, Event::SaveProfile);
        let deleting = matches!(event, Event::DeleteProfile);
        let draft = self.modules.deployment.profile.as_ref().filter(|_| {
            saving || duplicate || deleting || matches!(event, Event::ProfileProject(..))
        });
        let owner = draft.and_then(|draft| {
            self.workspace.spaces.iter().position(|space| {
                space
                    .deployments
                    .iter()
                    .any(|profile| profile.id == draft.id)
            })
        });
        if let Event::ProfileProject(index, ref directory) = event {
            if draft.is_none()
                || !self.workspace.spaces.get(index).is_some_and(|space| {
                    space
                        .projects
                        .iter()
                        .any(|project| &project.directory == directory)
                })
            {
                return Task::none();
            }
            self.modules.deployment.profile_destination = (owner != Some(index)).then_some(index);
        }
        let destination = draft.and_then(|draft| {
            self.modules
                .deployment
                .profile_destination
                .or(owner)
                .filter(|&index| {
                    self.workspace.spaces.get(index).is_some_and(|space| {
                        space
                            .projects
                            .iter()
                            .any(|project| project.directory == draft.project)
                    })
                })
                .or_else(|| {
                    self.workspace.spaces.iter().position(|space| {
                        space
                            .projects
                            .iter()
                            .any(|project| project.directory == draft.project)
                    })
                })
        });
        if (saving || duplicate) && destination.is_none() {
            self.modules.deployment.error = Some(crate::domain::deployment::Error::Configuration);
            return Task::none();
        }
        let source_id = draft.map(|draft| draft.id.clone());
        let index = if saving || duplicate {
            destination
        } else if deleting {
            owner
        } else {
            self.workspace.selected_space
        };
        let creating = matches!(event, Event::NewProfile | Event::DiscardChanges)
            || matches!(event, Event::ConfirmName)
                && self.modules.deployment.naming == Some(NameAction::NewProfile);
        let fallback = if creating && project.is_none() {
            self.workspace
                .spaces
                .iter()
                .find_map(|space| space.projects.first())
                .map(|project| project.directory.clone())
        } else {
            None
        };
        let mut temporary = Vec::new();
        let global_selection = matches!(event, Event::OpenProfiles | Event::EditProfile(_))
            || matches!(event, Event::DiscardChanges)
                && self.modules.deployment.has_settings_dialog(false);
        let profiles =
            if global_selection {
                temporary.extend(
                    self.workspace
                        .selected_space
                        .and_then(|index| self.workspace.spaces.get(index))
                        .into_iter()
                        .chain(self.workspace.spaces.iter().enumerate().filter_map(
                            |(index, space)| {
                                (self.workspace.selected_space != Some(index)).then_some(space)
                            },
                        ))
                        .flat_map(|space| space.deployments.iter().cloned()),
                );
                &mut temporary
            } else {
                index
                    .and_then(|index| self.workspace.spaces.get_mut(index))
                    .map_or(&mut temporary, |space| &mut space.deployments)
            };
        let (task, effect) = self.modules.deployment.update(
            event,
            &mut self.workspace.deployment,
            profiles,
            project.as_deref().or(fallback.as_deref()),
        );
        if matches!(effect, Effect::Save)
            && owner != destination
            && let Some(id) = source_id
            && let Some(index) = if saving {
                owner
            } else if duplicate && owner.is_some() {
                destination
            } else {
                None
            }
        {
            self.workspace.spaces[index]
                .deployments
                .retain(|profile| profile.id != id);
        }
        let task = task.map(deployment_message);
        match effect {
            Effect::None => task,
            Effect::OpenLocal(path) => Task::batch([task, self.open_file(path)]),
            Effect::CopyPath(path) => Task::batch([task, iced::clipboard::write(path)]),
            Effect::CompareRequested {
                profile,
                label,
                path,
                remote,
            } => Task::batch([
                task,
                self.begin_remote_comparison(profile, label, path, remote),
            ]),
            Effect::EditorSaved(token, result) => {
                let saved = self.update_editor(crate::app::editor::Event::Saved(
                    token,
                    result.map_err(|error| crate::modules::deployment::error_key(&error)),
                ));
                Task::batch([task, saved])
            }
            Effect::Transferred(name) => {
                self.notifications
                    .success(crate::i18n::Key::DeploymentComplete, Some(name));
                task
            }
            Effect::Save => Task::batch([task, self.save()]),
            Effect::Connections => Task::batch([
                task,
                self.update(Message::SelectSettingsTab(SettingsTab::Connections)),
            ]),
            Effect::Settings => Task::batch([
                task,
                self.update(Message::SelectSettingsTab(SettingsTab::Deployment)),
            ]),
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
