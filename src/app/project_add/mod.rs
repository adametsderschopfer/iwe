//! Keep project-source choices and clone requests separate from saved spaces.

use super::{App, Message as AppMessage, Navigation, spaces};
pub(crate) use crate::domain::git::CloneError as Error;
use crate::infrastructure::project_clone;
use iced::{Task, widget::operation};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct Draft {
    pub space: Option<usize>,
    pub cloning: bool,
    pub remote: String,
    pub destination: String,
    pub parent: PathBuf,
    pub custom_destination: bool,
    pub pending: bool,
    pub picking: bool,
    pub error: Option<project_clone::Error>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Open(Option<usize>),
    Directories,
    Remote,
    Url(String),
    Destination(String),
    PickParent,
    ParentPicked(u64, Option<PathBuf>),
    Clone,
    Finished(u64, Result<PathBuf, project_clone::Error>),
    Cancel,
}

impl App {
    pub(super) fn update_project_add(&mut self, message: Message) -> Task<AppMessage> {
        if let Message::Open(space) = message {
            if !self.loaded
                || self.picking
                || self.project_add.is_some()
                || self.auth_dialog.is_some()
                || self.git_actions.has_dialog()
                || space.is_some_and(|index| index >= self.workspace.spaces.len())
                || (space.is_none() && self.space_draft.is_none())
                || (space.is_some() && self.space_draft.is_some())
            {
                return Task::none();
            }
            let mut parents = BTreeMap::new();
            let directories: Vec<_> = if let Some(space) = space {
                self.workspace.spaces[space]
                    .projects
                    .iter()
                    .map(|project| &project.directory)
                    .collect()
            } else {
                self.space_draft
                    .as_ref()
                    .map(|draft| draft.directories.iter().collect())
                    .unwrap_or_default()
            };
            for directory in directories {
                if let Some(parent) = directory.parent()
                    && parent.is_absolute()
                    && parent.parent().is_some()
                {
                    *parents.entry(parent.to_owned()).or_insert(0usize) += 1;
                }
            }
            let maximum = parents.values().copied().max().unwrap_or(0);
            let mut candidates = parents.into_iter().filter(|(_, count)| *count == maximum);
            let first = candidates.next();
            let parent = if candidates.next().is_none() {
                first
                    .map(|(path, _)| path)
                    .unwrap_or_else(project_clone::default_parent)
            } else {
                project_clone::default_parent()
            };
            self.project_add = Some(Draft {
                space,
                cloning: false,
                remote: String::new(),
                destination: String::new(),
                parent,
                custom_destination: false,
                pending: false,
                picking: false,
                error: None,
            });
            self.space_menu = None;
            self.project_menu = None;
            self.repository_menu = None;
            self.tabs.menu = None;
            self.files.menu = None;
            self.git_actions.menu = None;
            self.git_actions.bulk.menu = None;
            self.space_request += 1;
            return Task::none();
        }
        let Some(draft) = self.project_add.as_mut() else {
            return Task::none();
        };
        if draft.pending && !matches!(message, Message::Finished(_, _))
            || draft.picking && !matches!(message, Message::ParentPicked(_, _) | Message::Cancel)
        {
            return Task::none();
        }
        match message {
            Message::Directories => {
                let space = draft.space;
                self.project_add = None;
                return self.update_space(spaces::Message::PickProject(space));
            }
            Message::Remote => {
                draft.cloning = true;
                return operation::focus("clone-url");
            }
            Message::Url(remote) => {
                draft.remote = remote;
                draft.error = None;
                if !draft.custom_destination {
                    draft.destination = project_clone::repository_name(&draft.remote)
                        .map(|name| draft.parent.join(name).to_string_lossy().into_owned())
                        .unwrap_or_default();
                }
            }
            Message::Destination(destination) => {
                draft.destination = destination;
                draft.custom_destination = true;
                draft.error = None;
            }
            Message::PickParent => {
                draft.picking = true;
                let request = self.space_request;
                return Task::perform(
                    project_clone::choose_parent(draft.parent.clone()),
                    move |parent| AppMessage::ProjectAdd(Message::ParentPicked(request, parent)),
                );
            }
            Message::ParentPicked(request, parent)
                if request == self.space_request && draft.picking =>
            {
                draft.picking = false;
                if let Some(parent) = parent {
                    draft.parent = parent;
                    draft.custom_destination = false;
                    let remote = draft.remote.clone();
                    return self.update_project_add(Message::Url(remote));
                }
            }
            Message::Clone if draft.cloning => {
                if project_clone::repository_name(&draft.remote).is_none() {
                    draft.error = Some(project_clone::Error::InvalidRemote);
                    return Task::none();
                }
                let destination = PathBuf::from(draft.destination.trim());
                if !destination.is_absolute() || destination.file_name().is_none() {
                    draft.error = Some(project_clone::Error::Destination);
                    return Task::none();
                }
                draft.pending = true;
                draft.error = None;
                self.space_request += 1;
                let request = self.space_request;
                return Task::perform(
                    project_clone::clone(draft.remote.clone(), destination),
                    move |result| AppMessage::ProjectAdd(Message::Finished(request, result)),
                );
            }
            Message::Finished(request, result)
                if request == self.space_request && draft.pending =>
            {
                draft.pending = false;
                match result {
                    Ok(directory) => {
                        let space = draft.space;
                        self.project_add = None;
                        if let Some(space) = space {
                            self.workspace.add_project(space, directory);
                            self.navigation = Navigation::Projects;
                            return Task::batch([self.load_directory(), self.save()]);
                        }
                        if let Some(draft) = self.space_draft.as_mut()
                            && !draft.directories.contains(&directory)
                        {
                            draft.directories.push(directory);
                        }
                    }
                    Err(error) => draft.error = Some(error),
                }
            }
            Message::Cancel => {
                self.project_add = None;
                self.space_request += 1;
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
